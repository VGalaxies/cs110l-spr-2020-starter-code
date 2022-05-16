use std::ops::Add;

#[derive(Debug, PartialEq, Clone, Copy)]
struct Point {
    x: f64,
    y: f64
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Point {x: x, y: y}
    }
}

pub trait ComputeNorm {
    fn compute_norm(&self) -> f64 {
        0.0 // default implementations
    }
}

impl ComputeNorm for Point {
    fn compute_norm(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}

impl Add for Point {
    type Output = Self; // associated type
    fn add(self, other: Self) -> Self {
        Point::new(self.x + other.x, self.y + other.y)
    }
}

fn main() {
    let the_origin = Point::new(0.0, 0.0);
    let mut p = the_origin; // copy semantics!
    println!("p: {:?}, the_origin: {:?}", p, the_origin);
    println!("are they equal? => {}", p == the_origin);
    p.x += 10.0;
    println!("p: {:?}, the_origin: {:?}", p, the_origin);
    println!("are they equal? => {}", p == the_origin);

    println!("norm of (3, 4) + the_origin: {}",
             (the_origin + Point::new(3.0, 4.0)).compute_norm());
}