use linked_list::LinkedList;
use linked_list::ComputeNorm;

pub mod linked_list;

fn test_list() {
    let mut list: LinkedList<u32> = LinkedList::new();
    assert!(list.is_empty());
    assert_eq!(list.get_size(), 0);
    for i in 1..4 {
        list.push_front(i);
    }
    println!("{}", list);
    println!("list size: {}", list.get_size());
    println!("top element: {}", list.pop_front().unwrap());
    println!("{}", list);
    println!("size: {}", list.get_size());
    println!("{}", list.to_string());

    for val in &list {
        println!("{}", val);
    }

    println!("size: {}", list.get_size());
    println!("{}", list.to_string());

    let mut iter = list.into_iter();
    assert_eq!(Some(2), iter.next());
    assert_eq!(Some(1), iter.next());
    assert_eq!(None, iter.next());
}

fn test_str_list() {
    let mut list: LinkedList<String> = LinkedList::new();
    list.push_front(String::from("World"));
    list.push_front(String::from("Hello"));
    println!("{}", list);
    println!("list size: {}", list.get_size());

    let cloned_list = list.clone();
    println!("{}", cloned_list.to_string());
    assert!(cloned_list == list);
    println!("top element: {}", list.pop_front().unwrap());
    println!("{}", list.to_string());
    println!("{}", cloned_list.to_string());
}

fn test_f64_list() {
    let mut list: LinkedList<f64> = LinkedList::new();
    list.push_front(3 as f64);
    list.push_front(4 as f64);
    println!("{}", list.compute_norm())
}


fn main() {
    test_list();
    test_str_list();
    test_f64_list();
}
