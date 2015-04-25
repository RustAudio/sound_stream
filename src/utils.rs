
use std::collections::VecDeque;

/// Take the given number of elements from the front of the VecDeque.
///
/// Fails if the num_elems given is higher than the length of the VecDeque.
pub fn take_front<T>(vec: &mut VecDeque<T>, num_elems: usize) -> Vec<T> {
    (0..num_elems).map(|_| vec.pop_front().unwrap()).collect()
}

