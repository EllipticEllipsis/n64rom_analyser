mod analysis;
mod compiler;
mod findcode;
// mod ipl3;
mod microcode;
mod utils;

use argh::FromArgs;
use std::io;
use utils::*;

const INSTRUCTION_SIZE: usize = 4;
const WORD_SIZE: usize = 4;
// const MIN_REGION_INSTRUCTIONS: usize = 4;
const SHOW_TRUE_RANGES: bool = false;

#[derive(FromArgs)]
/// Analyse a Nintendo 64 rom.
pub struct Args {
    #[argh(positional)]
    rom: String,

    // /// attempt to determine compiler
    // #[argh(switch, short = 'C')]
    // determine_compiler: bool,
    // /// how high to go
    // #[argh(option)]
    // height: usize,

    // /// an optional nickname for the pilot
    // #[argh(option)]
    // pilot_nickname: Option<String>,
}

use ::num_traits;

/// Rounds x up to the next multiple of n
fn round_up<T: num_traits::PrimInt>(x: T, n: T) -> T {
    (x / n) * n
}

/// Rounds x down to the previous multiple of n
fn round_down<T: num_traits::PrimInt>(x: T, n: T) -> T {
    (x / n) * n
}

fn main() -> io::Result<()> {
    let args = argh::from_env();

    let rom_bytes = read_rom(&args)?;

    let code_regions = findcode::find_code_regions(&rom_bytes);
    println!("Found {} code regions:", code_regions.len());

    for codeseg in code_regions {
        let start = round_up(codeseg.rom_start(), 0x10);
        let end = round_down(codeseg.rom_start(), 0x10);

        if !SHOW_TRUE_RANGES {
            print!(
                "  0x{:08X} to 0x{:08X} (0x{:06X}) rsp: {}\n",
                start,
                end,
                end - start,
                codeseg.has_rsp()
            );
        } else {
            print!(
                "  0x{:08X} to 0x{:08X} (0x{:06X}) rsp: {}\n",
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
