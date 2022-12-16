use crate::findcode;
use crate::findcode::analysis::MyInstruction;
use crate::findcode::RomRegion;
use crate::utils::read_be_word;
use crate::Args;
use crate::INSTRUCTION_SIZE;

pub enum Compiler {
    Unknown,
    IDO { version: Option<u32> }, // for now
    GCC { version: Option<u32> },
    SN64 { version: Option<u32> },
}

// pub fn heuristics(args: &Args) {

// }

pub fn b_vs_j(rom_bytes: &[u8], regions: &[RomRegion]) -> (i32, i32) {
    println!("Analysing b vs j counts:");

    let mut j_count = 0;
    let mut b_count = 0;
    for region in regions {
        let mut regional_j_count = 0;
        let mut regional_b_count = 0;

        for chunk in rom_bytes[region.rom_start()..region.rom_end()].chunks_exact(INSTRUCTION_SIZE)
        {
            let instr = MyInstruction::new(read_be_word(chunk));
            match instr.0.instr_id() {
                rabbitizer::InstrId::cpu_b => regional_b_count += 1,
                rabbitizer::InstrId::cpu_j => {
                    regional_j_count += 1;
                    println!("{:8X} ({})", instr.0.raw(), instr.0.disassemble(None, 0))
                }
                _ => (),
            }
        }
        println!(
            "[{:6X}, {:6X}): b: {regional_b_count:4}, j: {regional_j_count:4}",
            region.rom_start(),
            region.rom_end()
        );
        b_count += regional_b_count;
        j_count += regional_j_count;
    }
    println!("Total: b: {b_count:4} j: {j_count:4}");
    // Compiler::IDO { version: None }
    (b_count, j_count)
}
