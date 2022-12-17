use std::collections::HashSet;
use std::fmt::Display;
use std::io;
use strum::EnumCount;
use strum::IntoEnumIterator;
use strum_macros;
use strum_macros::EnumIter;

use crate::findcode;
use crate::utils::*;
use crate::Args;
use crate::INSTRUCTION_SIZE;
use findcode::analysis::*;
use findcode::RomRegion;

#[derive(EnumIter, Debug, PartialEq, Eq, Clone, Copy, Hash, strum_macros::EnumCount)]
pub enum Compiler {
    // Unknown, // Not yet known
    IDO53,  // IDO 5.3
    IDO71,  // IDO 7.1
    KMCGCC, // KMC GCC (2.7.2)
    ISGCC,  // Intelligent Systems GCC (2.8.1)
    SN64,   // SN64 (GCC 2.7/8 + custom assembler)
    SNCXX,  // SN64 C++ compiler
    MWCC,   // Metrowerks "CodeWarrior" N64 compiler
}

impl Display for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Compiler::IDO53 => "IDO 5.3",
                Compiler::IDO71 => "IDO 7.1",
                Compiler::KMCGCC => "KMC GCC",
                Compiler::ISGCC => "IS GCC",
                Compiler::SN64 => "SN64",
                Compiler::SNCXX => "SN64 C++",
                Compiler::MWCC => "CodeWarrior N64",
            }
        )
    }
}

/// Distinguish IDO from GCCs using unconditional branches (IDO uses b, GCC uses j)
pub fn b_vs_j(
    rom_bytes: &[u8],
    region: &RomRegion,
    possible_compilers: &mut HashSet<Compiler>,
) -> (i32, i32) {
    let mut j_count = 0;
    let mut b_count = 0;

    for chunk in rom_bytes[region.rom_start()..region.rom_end()].chunks_exact(INSTRUCTION_SIZE) {
        let instr = MyInstruction::new(read_be_word(chunk));
        match instr.0.instr_id() {
            rabbitizer::InstrId::cpu_b => {
                b_count += 1;
            }
            rabbitizer::InstrId::cpu_j => {
                j_count += 1;
            }
            _ => (),
        }
    }

    // Do not attempt to guess if there are too few
    if b_count + j_count > BJ_THRESHOLD {
        if b_count > j_count {
            possible_compilers.retain(|v| [Compiler::IDO53, Compiler::IDO71].contains(v));
        } else {
            possible_compilers.retain(|v| ![Compiler::IDO53, Compiler::IDO71].contains(v));
        }
    }

    (b_count, j_count)
}

/// KMC GCC (at least) will always load floats via
/// ```mips
/// lui $at []
/// ori $at, $at [] # (optional)
/// mtc1 $at, []
/// ```
/// SN64 will never do this.
/// IDO can insert other instructions between them (or will use rodata)
fn float_load_pattern(
    rom_bytes: &[u8],
    region: &RomRegion,
    possible_compilers: &mut HashSet<Compiler>,
) -> (i32, i32) {
    let mut last_was_lui_at = false;
    let mut float_load_pattern_count = 0;
    let mut isolated_mtc1_count = 0;

    for chunk in rom_bytes[region.rom_start()..region.rom_end()].chunks_exact(INSTRUCTION_SIZE) {
        let instr = MyInstruction::new(read_be_word(chunk));

        match instr.0.instr_id() {
            rabbitizer::InstrId::cpu_lui => {
                if instr.instr_get_rt() == MipsGpr::at {
                    last_was_lui_at = true;
                    continue;
                }
            }
            rabbitizer::InstrId::cpu_mtc1 => {
                if last_was_lui_at && instr.instr_get_rt() == MipsGpr::at {
                    float_load_pattern_count += 1;
                } else {
                    isolated_mtc1_count += 1;
                }
            }
            rabbitizer::InstrId::cpu_ori => {
                // ignore an intermediate ori $at, $at
                if instr.instr_get_rs() == MipsGpr::at && instr.instr_get_rt() == MipsGpr::at {
                    continue;
                }
            }
            _ => (),
        }

        last_was_lui_at = false;
    }

    if float_load_pattern_count > 0 {
        // Pattern found, could be GCC or IDO but not SN
        possible_compilers.retain(|v| ![Compiler::SN64, Compiler::SNCXX].contains(v));
    }

    if isolated_mtc1_count > 0 {
        // Must be IDO
        possible_compilers.retain(|v| [Compiler::IDO53, Compiler::IDO71].contains(v));
    }

    (float_load_pattern_count, isolated_mtc1_count)
}

const BJ_THRESHOLD: i32 = 10;

pub fn analyse(_args: &Args, rom_bytes: &[u8], regions: &[RomRegion]) -> io::Result<()> {
    // Start with all possible and narrow it down
    // let mut overall_possible_compilers = Compiler::iter().collect::<HashSet<_>>();
    let mut overall_possible_compilers = HashSet::<Compiler>::new();

    let mut total_b_count = 0;
    let mut total_j_count = 0;
    let mut total_float_load_pattern_count = 0;
    let mut total_isolated_mtc1_count = 0;

    for region in regions {
        let mut regional_possible_compilers = Compiler::iter().collect::<HashSet<_>>();

        let (b_count, j_count) = b_vs_j(&rom_bytes, region, &mut regional_possible_compilers);
        total_b_count += b_count;
        total_j_count += j_count;

        print!(
            "[{:7X}, {:7X}): b: {:4}, j: {:4}  ",
            region.rom_start(),
            region.rom_end(),
            b_count,
            j_count
        );

        let (float_load_pattern_count, isolated_mtc1_count) =
            float_load_pattern(&rom_bytes, &region, &mut regional_possible_compilers);
        total_float_load_pattern_count += float_load_pattern_count;
        total_isolated_mtc1_count += isolated_mtc1_count;

        print!(
            "lui-(ori)-mtc1: {:4}, isolated mtc1: {:4}  ",
            float_load_pattern_count, isolated_mtc1_count
        );

        
        if regional_possible_compilers.len() < Compiler::COUNT {
            overall_possible_compilers.extend(&regional_possible_compilers);
        }
        print!("Possible compilers: ");
        let mut compiler_list = regional_possible_compilers.into_iter().map(|v| format!("{:?}", v)).collect::<Vec<_>>();
        compiler_list.sort();
        println!("{{{}}}", compiler_list.join(","));
    }

    println!();
    println!("Total b/j: b: {:4}, j: {:4}", total_b_count, total_j_count);
    println!(
        "Total mtc1: lui-(ori)-mtc1: {:4}, isolated mtc1: {:4}",
        total_float_load_pattern_count, total_isolated_mtc1_count
    );
    println!();
    println!("Possible compilers: ");

    let mut compiler_list = overall_possible_compilers.into_iter().map(|v| format!("{}", v)).collect::<Vec<_>>();
    compiler_list.sort();
    print!("{}", compiler_list.join(", "));
    println!();

    Ok(())
}
