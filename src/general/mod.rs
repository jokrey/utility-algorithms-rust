use std::fmt;
use std::ops::Range;

use rand::seq::SliceRandom;
use std::fmt::{Display, Formatter};

pub fn sample_unique<T: std::iter::Step>(range: Range<T>, num: usize) -> Vec<T> {
    let mut sampled: Vec<T> = range.collect();
    sampled.shuffle(&mut rand::thread_rng());
    sampled.truncate(num);
    sampled
}


#[derive(Debug)]
pub struct Wrapper(Vec<u8>);
impl Wrapper {
    pub fn from(vec: Vec<u8>) -> Wrapper {
        Wrapper(vec)
    }
    pub fn raw(self) -> Vec<u8> {
        self.0
    }
}
impl fmt::Display for Wrapper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, v) in self.0.iter().enumerate() {
            if i == 0 {
                write!(f, "{}", v)?;
            } else {
                write!(f, " {}", v)?;
            }
        }
        Ok(())
    }
}
impl Clone for Wrapper {
    fn clone(&self) -> Self {
        Wrapper(self.0.clone())
    }
}


#[derive(Debug)]
pub struct DisplayableTuple<E: Display, T: Display>(E, T);
impl<E: Display, T: Display> DisplayableTuple<E, T> {
    pub fn new(e: E, t: T) -> DisplayableTuple<E, T> {
        DisplayableTuple(e, t)
    }
    pub fn from(tuple: (E, T)) -> DisplayableTuple<E, T> {
        DisplayableTuple(tuple.0, tuple.1)
    }
    pub fn raw(self) -> (E, T) {
        (self.0, self.1)
    }
    pub fn get_0(&self) -> &E {
        &self.0
    }
    pub fn get_1(&self) -> &T {
        &self.1
    }
}
impl<E: Display, T: Display> Display for DisplayableTuple<E, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}


pub fn get_bit_at(input: u8, k: u8) -> bool {
    if k < 8 {
        (input >> k) & 1 != 0
    } else {
        panic!("bit index out of bounds")
    }
}
pub fn set_bit_at(input: u8, k: u8) -> u8 {
    input | (1 << k)
}

pub fn print_bits(bytes: &[u8]) {
    for byte in bytes {
        print!("|");
        for i in 0..8 {
            if i == 7 {
                print!("{}", if get_bit_at(*byte, i) {1} else {0});
            } else {
                print!("{}, ", if get_bit_at(*byte, i) {1} else {0});
            }
        }
        print!("|");
    }
    println!();
}