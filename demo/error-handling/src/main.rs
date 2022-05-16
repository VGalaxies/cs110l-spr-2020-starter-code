extern crate rand;
use rand::Rng;

fn get_random_num() -> u32 {
    rand::thread_rng().gen_range(0, 42)
}

fn feeling_lucky() -> Option<String> {
    if get_random_num() > 10 {
        Some(String::from("I'm feeling lucky!"))
    } else {
        None
    }
}

fn poke_toddler() -> Result<&'static str, &'static str> {
    if get_random_num() > 10 {
        Ok("Hahahaha!")
    } else {
        Err("Waaaaahhh!")
    }
}

fn main() {
    if feeling_lucky().is_none() {
        println!("Not feeling lucky :(")
    }

    println!(
        "{}",
        feeling_lucky().unwrap_or(String::from("Not lucky :("))
    );

    match feeling_lucky() {
        Some(message) => {
            println!("Got message: {}", message)
        }
        None => {
            println!("No message returned :-/")
        }
    }

    match poke_toddler() {
        Ok(message) => println!("Toddler said: {}", message),
        Err(cry) => println!("Toddler cried: {}", cry),
    }

    // Panic if the baby cries:
    println!("{}", poke_toddler().unwrap());
    // Same thing, but print a more descriptive panic message:
    println!("{}", poke_toddler().expect("Toddler cried :("));

    panic!("Sad times!");
}
