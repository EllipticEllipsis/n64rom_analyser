use rayon::prelude::*;

use crate::WORD_SIZE;

pub const YAZ0: &[u8] = "Yaz0".as_bytes();
pub const YAY0: &[u8] = "Yay0".as_bytes();
pub const MIO0: &[u8] = "MIO0".as_bytes();

pub fn find_magic(rom_bytes: &[u8], magic: &[u8]) -> Vec<usize> {
    // As a file must at least start on 4, we can limit the search to multiples of 4
    // TODO: possibly up this to 0x10
    let mut found = (0..rom_bytes.len())
        .into_par_iter()
        .step_by(WORD_SIZE)
        .filter(|&x| rom_bytes[x..].starts_with(magic))
        .collect::<Vec<_>>();
    found.par_sort();

    // let mut found = Vec::new();
    // for i in (0..rom_bytes.len()).step_by(4) {
    //     if rom_bytes[i..].starts_with(magic) {
    //         found.push(i);
    //     }
    // }
    if found.len() > 0 {
        println!(
            "{} {} segments found",
            found.len(),
            String::from_utf8_lossy(magic)
        );
        print!("[");
        for (i, loc) in found.iter().enumerate() {
            if i % 8 == 0 {
                println!();
                print!("    ");
            }
            print!("{:6X}, ", loc)
        }
        println!();
        println!("]");
    }

    found
}

pub fn find_all(rom_bytes: &[u8]) {
    find_magic(rom_bytes, MIO0);
    find_magic(rom_bytes, YAZ0);
    find_magic(rom_bytes, YAY0);
}
