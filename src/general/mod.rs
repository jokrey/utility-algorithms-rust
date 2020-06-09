use std::fmt;
use std::ops::Range;

use rand::seq::SliceRandom;
use std::fmt::{Display, Formatter};

pub fn sample_unique<T: std::iter::Step>(range: Range<T>, num: usize) -> Vec<T> {
    let mut sampled: Vec<T> = range.collect();
    sampled.shuffle(&mut rand::thread_rng());
    sampled.truncate(num);
    // sampled.shrink_to_fit();
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

pub fn is_odd(int: u8) -> bool {
    int&1 != 0
}


pub struct BitIterator<'a> {
    bytes: &'a [u8],
    byte_index: usize,
    bit_index: u8
}
impl BitIterator<'_> {
    pub fn new(bytes: &[u8]) -> BitIterator {
        BitIterator { bytes, byte_index: 0, bit_index: 0 }
    }
}
impl Iterator for BitIterator<'_> {
    type Item = bool;
    fn next(&mut self) -> Option<bool> {
        if self.byte_index >= self.bytes.len() {
            None
        } else {
            let byte = self.bytes[self.byte_index];
            let bit = get_bit_at(byte, self.bit_index);
            self.bit_index += 1;
            if self.bit_index == 8 {
                self.byte_index += 1;
                self.bit_index = 0;
            }
            Some(bit)
        }
    }
}

trait Stackable<T>: Popable<T> + Pushable<T> {}
impl<T, E: Popable<T> + Pushable<T>> Stackable<T> for E {}

pub trait Popable<T> {
    fn can_pop(&self) -> bool;
    fn pop(&mut self) -> Option<T>;
    fn top(&mut self) -> Option<T>;
    fn delete_top(&mut self) -> bool;
}
pub trait Pushable<T> {
    fn can_push(&self) -> bool;
    fn push(&mut self, t: T) -> bool;
}
impl <T> Pushable<T> for Vec<T> {
    fn can_push(&self) -> bool { true }
    fn push(&mut self, t: T) -> bool {
        self.push(t);
        true
    }
}
pub struct StackSlice<'a, T> {
    buffer: &'a mut [T],
    current_index: usize
}
impl <T> StackSlice<'_, T> {
    pub fn new(buffer: & mut [T]) -> StackSlice<T> {
        StackSlice { buffer, current_index: 0 }
    }
    pub fn as_slice(&self) -> &[T] {
        &self.buffer[0..self.current_index]
    }
    pub fn len(&self) -> usize { self.current_index }
    pub fn capacity(&self) -> usize { self.buffer.len() }
    pub fn capacity_reached(&self) -> bool { self.len() >= self.capacity() }
    pub fn is_empty(&self) -> bool { self.current_index == 0 }
    pub fn clear(&mut self) {self.current_index = 0;}
    pub unsafe fn set_len(&mut self, new_len: usize) {self.current_index = new_len;}
}
impl <T: Copy> StackSlice<'_, T> {
    pub unsafe fn clear_range(&mut self, i1: usize, i2: usize) {
        //copy from i2 -> len, to i1 -> (len - i2), set_len((len - i2))
        self.buffer.copy_within(i2..self.len(), i1);
        self.set_len(self.len() - i2);
    }
}
impl <T> Pushable<T> for StackSlice<'_, T> {
    fn can_push(&self) -> bool { !self.capacity_reached() }
    fn push(&mut self, t: T) -> bool {
        if self.capacity_reached() {
            false
        } else {
            self.buffer[self.current_index] = t;
            self.current_index += 1;
            true
        }
    }
}
impl <T> Popable<T> for StackSlice<'_, T> {
    fn can_pop(&self) -> bool { !self.is_empty() }
    fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            self.current_index -= 1;
            unsafe {
                Some(core::ptr::read(self.buffer.as_ptr().add(self.current_index)))
            }
        }
    }
    fn top(&mut self) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            unsafe {
                Some(core::ptr::read(self.buffer.as_ptr().add(self.current_index)))
            }
        }
    }
    fn delete_top(&mut self) -> bool {
        if self.is_empty() {
            false
        } else {
            self.current_index -= 1;
            true
        }
    }
}


#[test]
fn test_stack_for_slice() {
    let mut original_array = [false; 8];
    let mut stack_slice = StackSlice::new(&mut original_array[..]);
    stack_slice.push(false);
    stack_slice.push(true);

    assert_eq!(Some(true), stack_slice.pop());
    assert_eq!(Some(false), stack_slice.pop());
    assert_eq!(false, stack_slice.can_pop());
    assert_eq!(None, stack_slice.pop());
}


pub struct BytesBuilder<'a> {
    bytes: &'a mut dyn Pushable<u8>,
    current_byte: u8,
    bit_index : u8
}
impl BytesBuilder<'_> {
    pub fn new(bytes: &mut dyn Pushable<u8>) -> BytesBuilder {
        BytesBuilder {bytes, current_byte: 0, bit_index: 0}
    }
}
impl Pushable<bool> for BytesBuilder<'_> {
    fn can_push(&self) -> bool { self.bytes.can_push() }
    fn push(&mut self, bit: bool) -> bool {
        if self.bytes.can_push() {
            if bit {
                self.current_byte = set_bit_at(self.current_byte, self.bit_index);
            } //else is auto unset
            self.bit_index += 1;

            if self.bit_index == 8 {
                self.bytes.push(self.current_byte);
                self.current_byte = 0;
                self.bit_index = 0;
            }

            true
        } else {
            false
        }
    }
}


pub fn distance(x: u8, y: u8) -> u8 {
    if x < y {
        y - x
    } else {
        x - y
    }
}