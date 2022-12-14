mod analysis;
mod compiler;
mod findcode;
// mod ipl3;
mod microcode;
mod utils;

use argh::FromArgs;
use std::{fs, io};
use utils::*;

const INSTRUCTION_SIZE: usize = 4;
const WORD_SIZE: usize = 4;
// const MIN_REGION_INSTRUCTIONS: usize = 4;
const SHOW_TRUE_RANGES: bool = false;

#[derive(FromArgs)]
/// Analyse a Nintendo 64 rom.
pub struct Args {
    /// romfile to read
    #[argh(positional)]
    rom: String,

    // /// end of search, expect hex
    // #[argh(option)]
    // end: Option<String>,

    // /// attempt to determine compiler
    // #[argh(switch, short = 'C')]
    // determine_compiler: bool,
}

fn read_rom(args: &Args) -> io::Result<Vec<u8>> {
    let mut rom_bytes = fs::read(&args.rom)?;
    // // Bad but easier for now than using more modules
    // let end = 0xB80000;
    // rom_bytes.truncate(end);
    // rom_bytes.shrink_to_fit();

    let endian = get_endian(&rom_bytes)?;
    reend_array(&mut rom_bytes, &endian);

    Ok(rom_bytes)
}

fn main() -> io::Result<()> {
    let args = argh::from_env();

    let rom_bytes = read_rom(&args)?;

    let code_regions = findcode::find_code_regions(&rom_bytes);
    println!(
        "Found {} code region{}:",
        code_regions.len(),
        if code_regions.len() > 1 { "s" } else { "" }
    );

    for codeseg in code_regions {
        let start = round_down(codeseg.rom_start(), 0x10);
        let end = round_up(codeseg.rom_end(), 0x10);

        if !SHOW_TRUE_RANGES {
            println!(
                "  0x{:08X} to 0x{:08X} (0x{:06X}) rsp: {}",
                start,
                end,
                end - start,
                codeseg.has_rsp()
            );
        } else {
            println!(
                "  0x{:08X} to 0x{:08X} (0x{:06X}) rsp: {}",
                codeseg.rom_start(),
                codeseg.rom_end(),
                codeseg.rom_end() - codeseg.rom_start(),
                codeseg.has_rsp()
            );
            if codeseg.rom_start() != start {
                print!("    Warn: code region doesn't start at 16 byte alignment");
            }
        }
    }

    // if args.determine_compiler {
    //     compiler::heuristics(&args);
    // }

    Ok(())
}
