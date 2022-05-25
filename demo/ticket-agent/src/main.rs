extern crate rand;
use std::sync::{Arc, Mutex};

use std::{thread, time};
use rand::Rng;

fn handleCall() {
    let mut rng = rand::thread_rng();
    thread::sleep(time::Duration::from_millis(rng.gen_range(0, 10)));
}

fn takeBreak() {
    let mut rng = rand::thread_rng();
    thread::sleep(time::Duration::from_millis(rng.gen_range(0, 10)));
}

fn shouldTakeBreak() -> bool {
    rand::random()
}

fn ticketAgent(id: usize, remainingTickets: Arc<Mutex<usize>>) {
    loop {
        let mut remainingTicketsRef = remainingTickets.lock().unwrap();
        if (*remainingTicketsRef == 0) {
            break;
        }
        handleCall();
        *remainingTicketsRef -= 1;
        println!("Agent #{} sold a ticket! ({} more to be sold)",
            id, *remainingTicketsRef);
        if shouldTakeBreak() {
            takeBreak();
        }
    }
    println!("Agent #{} notices all tickets are sold, and goes home!", id);
}

fn main() {
    let remainingTickets: Arc<Mutex<usize>> = Arc::new(Mutex::new(250));

    let mut threads = Vec::new();
    for i in 0..10 {
        let remainingTicketsRef = remainingTickets.clone();
        threads.push(thread::spawn(move || {
            ticketAgent(i, remainingTicketsRef);
        }));
    }
    // wait for all the threads to finish
    for handle in threads {
        handle.join().expect("Panic occurred in thread!");
    }
    println!("End of business day!");
}

