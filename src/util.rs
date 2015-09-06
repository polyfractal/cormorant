
use std::io::Write;

#[inline(never)]
pub fn slice_copy(from: &[u8], mut to: &mut [u8]) -> usize {
    to.write(&from).unwrap()
}
