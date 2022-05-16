use std::fmt;
pub struct MatchingPair<T> {
    first: T,
    second: T
}

impl<T> MatchingPair<T> {
    pub fn new(first: T, second: T) -> Self {
        MatchingPair {first: first, second: second}
    }
}

impl<T> fmt::Display for MatchingPair<T> where T: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.first, self.second)
    }
}

impl <T: Clone> Clone for MatchingPair<T> {
    fn clone(&self) -> Self {
        Self {
            first: self.first.clone(),
            second: self.second.clone()
        }
    }
}

pub enum MyOption<T> {
    Sumthin(T), Nuthin
}

impl<T> fmt::Display for MyOption<T> where T: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MyOption::Sumthin(num) => write!(f, "Sumthin({})", num),
            MyOption::Nuthin => write!(f, "Nuthin :("),
        }
    }
}

fn identity_fn<T>(x: T) -> T {x}

// An example of trait composition -- T must impl Display and PartialOrd
fn print_min<T: fmt::Display + PartialOrd>(x: T, y: T) {
    if x < y {
        println!("The minimum is {}", x);
    } else {
        println!("The minimum is {}", y)
    }
}

fn main() {
    let ps_in_a_pod: MatchingPair<char> = MatchingPair::new('p', 'P');
    println!("two ps in a pod: {}", ps_in_a_pod);
    let my_some_five: MyOption<u32> = MyOption::Sumthin(5);
    let my_nuthin: MyOption<u32> = MyOption::Nuthin;
    println!("my_some_five: {}", my_some_five);
    println!("my_nuthin: {}", my_nuthin);

    let cloned = ps_in_a_pod.clone();
    println!("cloned: {}", cloned);

    println!("{:?}", identity_fn(Some(42)));
    print_min(2, 4);
}