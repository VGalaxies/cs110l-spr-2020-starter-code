extern crate crossbeam;
use std::io::BufRead;
use std::{thread, time};

fn parallel_map<T, U, F>(input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    output_vec.resize_with(input_vec.len(), Default::default);

    // TODO: implement parallel map!
    let (sender, receiver) = crossbeam::channel::unbounded();
    let (sender_re, receiver_re) = crossbeam::channel::unbounded();

    let mut threads = Vec::with_capacity(num_threads);
    for _ in 0..num_threads {
        let receiver = receiver.clone();
        let sender_re = sender_re.clone();
        threads.push(thread::spawn(move || {
            while let Ok(next_elem) = receiver.recv() {
                let (index, elem) = next_elem;
                sender_re
                    .send((index, f(elem)))
                    .expect("Tried writing to channel, but there are no receivers!");
            }
        }));
    }

    let mut index = 0;
    for elem in input_vec {
        sender
            .send((index, elem))
            .expect("Tried writing to channel, but there are no receivers!");
        index = index + 1;
    }

    drop(sender);

    for thread in threads {
        thread.join().expect("Panic occurred in thread");
    }

    drop(sender_re);

    while let Ok(next_elem) = receiver_re.recv() {
        let (index, elem) = next_elem;
        output_vec[index] = elem;
    }

    output_vec
}

fn main() {
    // let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let mut v = Vec::new();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let num = line.unwrap().parse::<u32>().unwrap();
        v.push(num);
    }

    let squares = parallel_map(v.clone(), 1000, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });
    println!("squares: {:?}", squares);

    let squares: Vec<u32> = v
        .iter()
        .map(|num| {
            println!("{} squared is {}", num, num * num);
            thread::sleep(time::Duration::from_millis(500));
            num * num
        })
        .collect();
    println!("squares: {:?}", squares);
}
