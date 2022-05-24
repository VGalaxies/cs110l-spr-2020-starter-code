extern crate crossbeam;
use std::io::BufRead;
use std::thread;

fn is_prime(num: u32) -> bool {
    if num <= 1 {
        return false;
    }
    for factor in 2..((num as f64).sqrt().floor() as u32) {
        if num % factor == 0 {
            return false;
        }
    }
    true
}

fn factor_number(num: u32) {
    if num == 1 || is_prime(num) {
        println!("{} = {}", num, num);
        return;
    }

    let mut factors = Vec::new();
    let mut curr_num = num;
    for factor in 2..num {
        while curr_num % factor == 0 {
            factors.push(factor);
            curr_num /= factor;
        }
    }
    factors.sort();
    let factors_str = factors
        .into_iter()
        .map(|f| f.to_string())
        .collect::<Vec<String>>()
        .join(" * ");
    println!("{} = {}", num, factors_str);
}

fn main() {
    let (sender, receiver) = crossbeam::channel::unbounded();

    let mut threads = Vec::new();
    for _ in 0..num_cpus::get() {
        let receiver = receiver.clone();
        threads.push(thread::spawn(move || {
            while let Ok(next_num) = receiver.recv() {
                factor_number(next_num);
            }
        }));
    }

    let stdin = std::io::stdin();

    for line in stdin.lock().lines() {
        let num = line.unwrap().parse::<u32>().unwrap();
        sender
            .send(num)
            .expect("Tried writing to channel, but there are no receivers!");
    }

    drop(sender);

    for thread in threads {
        thread.join().expect("Panic occurred in thread");
    }
}