use std::fmt::Display;

use rayon::prelude::*;

use crate::WORD_SIZE;

// pub const YAZ0: &[u8] = "Yaz0".as_bytes();
// pub const YAY0: &[u8] = "Yay0".as_bytes();
// pub const MIO0: &[u8] = "MIO0".as_bytes();

#[derive(Debug, Clone, Copy)]
pub enum Type {
    MIO0,
    Yaz0,
    Yay0,
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Type::MIO0 => "MIO0",
                Type::Yaz0 => "Yaz0",
                Type::Yay0 => "Yay0",
                // _ => unimplemented!(),
            }
        )
    }
}

impl Type {
    const fn magic(&self) -> &[u8] {
        match self {
            Type::MIO0 => "MIO0",
            Type::Yaz0 => "Yaz0",
            Type::Yay0 => "Yay0",
            // _ => unimplemented!(),
        }
        .as_bytes()
    }
}

pub struct CompressedSegment {
    algorithm: Type,
    rom_start: usize,
}

pub fn find_magic(rom_bytes: &[u8], algorithm: Type) -> Vec<CompressedSegment> {
    // As a file must at least start on 4, we can limit the search to multiples of 4
    // TODO: possibly up this to 0x10
    let magic = algorithm.magic();
    let mut found = (0..rom_bytes.len())
        .into_par_iter()
        .step_by(WORD_SIZE)
        .filter_map(|x| {
            if rom_bytes[x..].starts_with(magic) {
                Some(CompressedSegment {
                    algorithm,
                    rom_start: x,
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    found.sort_unstable_by_key(|k| k.rom_start);

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
        for (i, seg) in found.iter().enumerate() {
            if i % 8 == 0 {
                println!();
                print!("    ");
            }
            print!("{:6X} ({}), ", seg.rom_start, seg.algorithm);
        }
        println!();
        println!("]");
    }

    found
}

pub fn find_all(rom_bytes: &[u8]) {
    let mut found = find_magic(rom_bytes, Type::MIO0);
    found.append(&mut find_magic(rom_bytes, Type::Yaz0));
    found.append(&mut find_magic(rom_bytes, Type::Yay0));
}
