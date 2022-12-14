use crate::{INSTRUCTION_SIZE, WORD_SIZE};
use std::io;

#[derive(Debug, Clone, Copy)]
pub enum Endian {
    Good,
    Bad,
    Ugly,
}

pub fn get_endian(input: &[u8]) -> io::Result<Endian> {
    match &input[0..WORD_SIZE] {
        [0x80, 0x37, 0x12, 0x40] => Ok(Endian::Good),
        [0x40, 0x12, 0x37, 0x80] => Ok(Endian::Bad),
        [0x37, 0x80, 0x40, 0x12] => Ok(Endian::Ugly),
        _ => panic!("Unrecognised header format"),
    }
}

/// Re-ends an array in-place
pub fn reend_array(v: &mut [u8], endian: &Endian) {
    let n = v.len();
    assert!(n % INSTRUCTION_SIZE == 0);
    match endian {
        Endian::Good => (),
        Endian::Bad => {
            for chunk in v.chunks_exact_mut(WORD_SIZE) {
                chunk.reverse();
            }
        }
        Endian::Ugly => {
            for chunk in v.chunks_exact_mut(2) {
                chunk.reverse();
            }
        }
    };
}


use ::num_traits;

/// Rounds x up to the next multiple of n
pub fn round_up<T: num_traits::PrimInt>(x: T, n: T) -> T {
    ((x + n - T::one()) / n) * n
}

/// Rounds x down to the previous multiple of n
pub fn round_down<T: num_traits::PrimInt>(x: T, n: T) -> T {
    (x / n) * n
}

pub fn read_be_word(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(bytes[0..WORD_SIZE].try_into().unwrap())
}
