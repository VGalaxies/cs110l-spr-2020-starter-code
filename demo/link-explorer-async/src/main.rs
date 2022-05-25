extern crate reqwest;
extern crate select;
#[macro_use]
extern crate error_chain;

use select::document::Document;
use select::predicate::Name;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tokio::task;

error_chain! {
   foreign_links {
       ReqError(reqwest::Error);
       IoError(std::io::Error);
   }
}

struct Article {
    url: String,
    len: usize,
}

const BATCH_SIZE: usize = 60;

async fn get_body_text(link: &String, connection_permits: Arc<Semaphore>) -> Result<String> {
    let _permit = connection_permits.acquire().await;
    // Once the permit is dropped, it will increment the semaphore
    let body = reqwest::get(link)?.text()?;
    Ok(body)
}

// https://rust-lang-nursery.github.io/rust-cookbook/web/scraping.html
#[tokio::main(worker_threads = 20)]
async fn main() -> Result<()> {
    let body =
        reqwest::get("https://en.wikipedia.org/wiki/Multithreading_(computer_architecture)")?
            .text()?;
    // Identify all linked wikipedia pages
    let links = Document::from_read(body.as_bytes())?
        .find(Name("a"))
        .filter_map(|n| {
            if let Some(link_str) = n.attr("href") {
                if link_str.starts_with("/wiki/") {
                    Some(format!("{}/{}", "https://en.wikipedia.org", &link_str[1..]))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<String>>();
    // println!("links: {:?}", links);
    let longest_article = Arc::new(Mutex::new(Article {
        url: "".to_string(),
        len: 0,
    }));
    let connection_permits = Arc::new(Semaphore::new(BATCH_SIZE));

    for link in &links {
        let longest_article_clone = longest_article.clone();
        let link_clone = link.clone();
        let connection_permits_clone = connection_permits.clone();
        task::spawn(async move {
            if let Ok(body) = get_body_text(&link_clone, connection_permits_clone).await {
                let curr_len = body.len();
                let mut longest_article_ref = longest_article_clone.lock().await;
                if curr_len > longest_article_ref.len {
                    longest_article_ref.len = curr_len;
                    longest_article_ref.url = link_clone.to_string();
                }
            }
        })
        .await;
    }

    let longest_article_ref = longest_article.lock().await;
    println!(
        "{} was the longest article with length {}",
        longest_article_ref.url, longest_article_ref.len
    );
    Ok(())
}
