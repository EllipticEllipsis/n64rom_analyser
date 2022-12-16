
mod compiler;
mod findcode;
// mod ipl3;
mod utils;

use argh::FromArgs;
use parse_int;
use std::{
    fs::{self, File},
    io::{self, Read},
};
use utils::*;

const INSTRUCTION_SIZE: usize = 4;
const WORD_SIZE: usize = 4;

const IPL3_END: usize = 0x1000;

// const MIN_REGION_INSTRUCTIONS: usize = 4;
const SHOW_TRUE_RANGES: bool = false;

fn parse_number(input: &str) -> Result<usize, String> {
    parse_int::parse::<usize>(input).map_err(|_| input.to_string())
}

#[derive(FromArgs)]
/// Analyse a Nintendo 64 rom.
pub struct Args {
    /// romfile to read
    #[argh(positional)]
    rom: String,

    // Could implement `start`, but fiddlier and less useful.
    /// end of search, expect hex
    #[argh(option, from_str_fn(parse_number))]
    end: Option<usize>,
    // /// attempt to determine compiler
    // #[argh(switch, short = 'C')]
    // determine_compiler: bool,
}

fn configure_rabbitizer() {
    rabbitizer::config_set_treat_j_as_unconditional_branch(true);
}

fn read_rom(args: &Args) -> io::Result<Vec<u8>> {
    let mut rom_bytes = Vec::with_capacity(0x100000);

    let f = File::open(&args.rom)?;

    if let Some(end) = args.end {
        let mut handle = f.take(end as u64);
        handle.read(&mut rom_bytes)?;
        println!("Examining range {:#08X}-{:#08X}", 0, end);
    } else {
        rom_bytes = fs::read(&args.rom)?;
        println!("Examining full rom, range {:#08X}-{:#08X}", 0, rom_bytes.len());
    }

    let endian = get_endian(&rom_bytes)?;
    reend_array(&mut rom_bytes, &endian);

    Ok(rom_bytes)
}

fn main() -> io::Result<()> {
    configure_rabbitizer();

    // Process arguments
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
