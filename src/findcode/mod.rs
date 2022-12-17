pub mod analysis;
pub mod microcode;

use std::fmt::Display;

use analysis::MipsGpr;
use analysis::MyInstruction;
use crate::utils::*;
use crate::INSTRUCTION_SIZE;
use crate::IPL3_END;

#[derive(Debug)]
pub struct RomRegion {
    rom_start: usize,
    rom_end: usize,
    has_rsp: bool,
}

impl RomRegion {
    pub fn new(rom_start: usize, rom_end: usize) -> Self {
        Self {
            rom_start,
            rom_end,
            has_rsp: false,
        }
    }

    pub fn rom_start(&self) -> usize {
        self.rom_start
    }
    pub fn rom_end(&self) -> usize {
        self.rom_end
    }
    pub fn has_rsp(&self) -> bool {
        self.has_rsp
    }
    pub fn set_rom_start(&mut self, rom_start: usize) {
        self.rom_start = rom_start;
    }
    pub fn set_rom_end(&mut self, rom_end: usize) {
        self.rom_end = rom_end;
    }
    pub fn set_has_rsp(&mut self, has_rsp: bool) {
        self.has_rsp = has_rsp;
    }
}

impl Display for RomRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{:6X}, {:6X}) ({})",
            self.rom_start(),
            self.rom_end(),
            self.has_rsp()
        )
    }
}

fn is_unused_n64_instruction(id: rabbitizer::InstrId) -> bool {
    matches!(
        id,
        rabbitizer::InstrId::cpu_ll
            | rabbitizer::InstrId::cpu_sc
            | rabbitizer::InstrId::cpu_lld
            | rabbitizer::InstrId::cpu_scd
            | rabbitizer::InstrId::cpu_syscall
    )
}

/// Check if a given instruction is valid via several metrics
pub fn is_valid(my_instruction: &MyInstruction) -> bool {
    let id = my_instruction.0.instr_id();

    // Check for instructions with invalid bits or invalid opcodes
    if !rabbitizer::Instruction::is_valid(&my_instruction.0)
        || id == rabbitizer::InstrId::cpu_INVALID
    {
        // println!("Invalid instruction: {:08X}", my_instruction.0.raw());
        // println!("    {:08X} ({})", my_instruction.0.raw(), my_instruction.0.disassemble(None, 0));
        return false;
    }

    let is_store = my_instruction.0.does_store();
    let is_load = my_instruction.0.does_load();

    // Check for loads or stores with an offset from $zero
    if (is_store || is_load) && (my_instruction.instr_get_rs() == MipsGpr::zero) {
        // println!("Loads or stores with an offset from $zero");
        return false;
    }

    // This check is disabled as some compilers can generate load to $zero for a volatile dereference
    // Check for loads to $zero
    // let is_float = my_instruction.0.is_float();
    // if is_load && !is_float && my_instruction.instr_get_rt() == MipsGpr::zero {
    //     return false;
    // }

    // Check for arithmetic that outputs to $zero
    if my_instruction.0.modifies_rd() && my_instruction.instr_get_rd() == MipsGpr::zero {
        return false;
    }
    if my_instruction.0.modifies_rt() && my_instruction.instr_get_rt() == MipsGpr::zero {
        return false;
    }

    // Check for mtc0 or mfc0 with invalid registers
    if matches!(
        id,
        rabbitizer::InstrId::cpu_mtc0 | rabbitizer::InstrId::cpu_mfc0
    ) && my_instruction.instr_get_cop0_rd().is_err()
    {
        // println!(
        //     "mtc0 or mfc0 with invalid registers: {} ({:08X})",
        //     my_instruction.instr_get_cop0_rd().unwrap_err(),
        //     my_instruction.0.raw()
        // );
        return false;
    }

    // Check for instructions that wouldn't be in an N64 game, despite being valid
    if is_unused_n64_instruction(id) {
        // println!("Valid but not in N64");
        return false;
    }

    // Check for cache instructions with invalid parameters
    if id == rabbitizer::InstrId::cpu_cache {
        let cache_param = my_instruction.instr_get_op();
        let cache_op = cache_param >> 2;
        let cache_type = cache_param & 0x3;

        // Only cache operations 0-6 and cache types 0-1 are valid
        if cache_op > 6 || cache_type > 1 {
            // println!("Cache instructions with invalid parameters");
            return false;
        }
    }

    // Check for cop2 instructions, which are invalid for the N64's CPU
    if matches!(
        id,
        rabbitizer::InstrId::cpu_lwc2
            | rabbitizer::InstrId::cpu_ldc2
            | rabbitizer::InstrId::cpu_swc2
            | rabbitizer::InstrId::cpu_sdc2
    ) {
        // println!("cop2");
        return false;
    }

    // Check for trap instructions
    if my_instruction.0.is_trap() {
        // println!("trap");
        return false;
    }

    // Check for ctc0 and cfc0, which aren't valid on the N64
    if matches!(
        id,
        rabbitizer::InstrId::cpu_ctc0 | rabbitizer::InstrId::cpu_cfc0
    ) {
        // println!("ctc0 or cfc0");
        return false;
    }

    // Check for instructions that don't exist on the N64's CPU
    if matches!(id, rabbitizer::InstrId::cpu_pref) {
        // println!("does not exist on the N64's CPU");
        return false;
    }

    true
}

fn is_valid_bytes(bytes: &[u8]) -> bool {
    let my_instruction = MyInstruction::new(read_be_word(bytes));
    is_valid(&my_instruction)
}

const JR_RA: u32 = 0x03E00008;

/// Search a span for any instances of the instruction `jr $ra`
fn find_return_locations(rom_bytes: &[u8]) -> Vec<usize> {
    // let locations = rom_bytes[IPL3_END..]
    //     .chunks_exact(INSTRUCTION_SIZE)
    //     .enumerate()
    //     .filter(|(_, v)| read_be_word(*v) == JR_RA)
    //     .map(|(index, _)| IPL3_END + INSTRUCTION_SIZE * index);

    // let next_is_valid_cpu = |loc: usize| is_valid_bytes(&rom_bytes[loc + 4..]);
    // let next_is_valid_rsp = |loc: usize| microcode::is_valid(&rom_bytes[loc + 4..]);

    // let filtered_locations = locations
    //     .filter(|&x| next_is_valid_cpu(x) || next_is_valid_rsp(x))
    //     .collect::<Vec<_>>();

    let mut filtered_locations = Vec::new();
    let mut iter = rom_bytes[IPL3_END..]
        .chunks_exact(INSTRUCTION_SIZE)
        .enumerate();
    while let Some((i, chunk)) = iter.next() {
        if read_be_word(chunk) == JR_RA {
            if let Some((_, chunk)) = iter.next() {
                if is_valid_bytes(chunk) || microcode::is_valid_bytes(chunk) {
                    filtered_locations.push(INSTRUCTION_SIZE * i + IPL3_END);
                }
            }
        }
    }
    // println!("locations:");
    // print!("[");
    // for (i, loc) in output.iter().enumerate() {
    //     if i % 0x10 == 0 {
    //         print!("\n   ");
    //     }
    //     print!("{loc:6X}, ")
    // }
    // println!("");
    // println!("]");
    // println!("{}", output.len());

    filtered_locations
}

/// Searches backwards from the given rom address until it hits an invalid instruction
fn find_code_start(rom_bytes: &[u8], rom_addr: usize) -> usize {
    // IPL3_END
    //     + INSTRUCTION_SIZE
    //         * rom_bytes[IPL3_END..rom_addr]
    //             .chunks_exact(INSTRUCTION_SIZE)
    //             .rposition(|v| !is_valid_bytes(v))
    //             .unwrap_or(0)
    let mut r = rom_addr;
    // println!("start initial {r:6X}");
    while r > IPL3_END {
        let cr = r - INSTRUCTION_SIZE;
        if !is_valid_bytes(&rom_bytes[cr..]) {
            break;
        }
        r = cr;
    }
    // println!("start {r:6X}");
    return r;
}

/// Searches forwards from the given rom address until it hits an invalid instruction
fn find_code_end(rom_bytes: &[u8], rom_addr: usize) -> usize {
    // rom_addr
    //     + INSTRUCTION_SIZE
    //         * rom_bytes[rom_addr..]
    //             .chunks_exact(INSTRUCTION_SIZE)
    //             .position(|v| !is_valid_bytes(v))
    //             .unwrap_or(rom_bytes.len())

    let mut r = rom_addr;
    // println!("end initial {r:6X}");
    while r > 0 {
        if !is_valid_bytes(&rom_bytes[r..]) {
            break;
        }
        r += INSTRUCTION_SIZE;
    }
    // println!("end {r:6X}");
    return r;
}

/// Check if a given instruction word is an unconditional non-linking branch (i.e. `b`, `j`, or `jr`)
fn is_unconditional_branch(bytes: &[u8]) -> bool {
    let instr = rabbitizer::Instruction::new(read_be_word(bytes), 0);

    matches!(
        instr.instr_id(),
        rabbitizer::InstrId::cpu_b | rabbitizer::InstrId::cpu_j | rabbitizer::InstrId::cpu_jr
    )
}

/// Trims zeroes from the start of a code region and "loose" instructions from the end
fn trim_region(codeseg: &mut RomRegion, rom_bytes: &[u8]) {
    let mut start = codeseg.rom_start();
    let mut end = codeseg.rom_end();
    let invalid_start_count = analysis::count_invalid_start_instructions(codeseg, rom_bytes);

    start += invalid_start_count * INSTRUCTION_SIZE;

    // Remove leading nops
    start += INSTRUCTION_SIZE
        * &rom_bytes[start..]
            .chunks_exact(INSTRUCTION_SIZE)
            .position(|v| read_be_word(v) != 0)
            .unwrap_or(0);

    // Any instruction that isn't eventually followed by an unconditional non-linking branch (b, j, jr) would run into
    // invalid code, so scan backwards until we see an unconditional branch and remove anything after it.
    // Scan two instructions back (8 bytes before the end) instead of one to include the delay slot.
    while !is_unconditional_branch(&rom_bytes[end - 2 * INSTRUCTION_SIZE..]) && end > start {
        end -= INSTRUCTION_SIZE;
    }

    codeseg.set_rom_start(start);
    codeseg.set_rom_end(end);
}

/// Check if a given rom range is valid CPU instructions
fn check_range(start: usize, end: usize, rom_bytes: &[u8]) -> bool {
    let mut prev_chunk = None;
    let mut identical_count = 0;

    for chunk in rom_bytes[start..end].chunks_exact(INSTRUCTION_SIZE) {
        // Check if the previous instruction is identical to this one
        if Some(chunk) == prev_chunk {
            // If it is, increase the consecutive identical instruction count
            identical_count += 1;
        } else {
            // Otherwise, reset the count and update the previous instruction for tracking
            prev_chunk = Some(chunk);
            identical_count = 0;
        }

        let instr = MyInstruction::new(read_be_word(chunk));
        // If there are 3 identical loads or stores in a row, it's not likely to be real code
        // Use 3 as the count because 2 could be plausible if it's a duplicated instruction by the compiler.
        // Only check for loads and stores because arithmetic could be duplicated to avoid more expensive operations,
        // e.g. x + x + x instead of 3 * x.
        if (identical_count >= 3) && (instr.0.does_load() || instr.0.does_store()) {
            return false;
        }
        if !is_valid(&instr) {
            return false;
        }
    }
    true
}

pub fn find_code_regions(rom_bytes: &[u8]) -> Vec<RomRegion> {
    let mut regions = Vec::with_capacity(0x400);
    let return_addrs = find_return_locations(rom_bytes);

    // let mut it = return_addrs.iter();
    let mut i = 0;

    while let Some(&cur) = return_addrs.get(i) {
        // println!("");
        // println!("index: {i}, it: {cur:X}");
        let region_start = find_code_start(rom_bytes, cur);
        let region_end = find_code_end(rom_bytes, cur);
        regions.push(RomRegion::new(region_start, region_end));

        // println!("{:?}", regions);

        while let Some(&cur) = return_addrs.get(i) {
            if cur >= regions.last().unwrap().rom_end() {
                break;
            }
            i += 1;
        }

        // for region in &regions {
        //     println!("{}", region);
        // }
        // println!("Trim");
        trim_region(regions.last_mut().unwrap(), rom_bytes);
        // for region in &regions {
        //     println!("{}", region);
        // }

        // If the current region is close enough to the previous region, check if there's valid RSP microcode between the two
        let len = regions.len();
        if len > 1 {
            let last_start = regions.last().unwrap().rom_start();
            let penultimate = regions.get_mut(len - 2).unwrap();
            // println!("{last_start:X}, {:X}", penultimate.rom_end());
            if last_start - penultimate.rom_end() < microcode::CHECK_THRESHHOLD {
                println!("Check for ucode");
                // Check if there's a range of valid CPU instructions between these two regions
                let mut valid_range = check_range(penultimate.rom_end(), last_start, rom_bytes);

                // If there isn't check for RSP instructions
                if !valid_range {
                    valid_range =
                        microcode::check_range(penultimate.rom_end(), last_start, rom_bytes);
                    // If RSP instructions were found, mark the first region as having RSP instructions
                    if valid_range {
                        penultimate.set_has_rsp(true);
                    }
                }
                if valid_range {
                    let new_end = regions.last().unwrap().rom_end();
                    regions.pop();
                    regions.last_mut().unwrap().set_rom_end(new_end);
                }
            }
        }

        // If the region has microcode, search forward until valid RSP instructions end
        if regions.last().unwrap().has_rsp() {
            // println!("Has rsp.");
            // Keep advancing the region's end until either the stop point is reached or something
            // that isn't a valid RSP instruction is seen
            let mut cur_end = regions.last().unwrap().rom_end();
            while regions.last().unwrap().rom_end() < rom_bytes.len()
                && microcode::is_valid_bytes(&rom_bytes[cur_end..])
            {
                // cur_end += INSTRUCTION_SIZE;
                regions
                    .last_mut()
                    .unwrap()
                    .set_rom_end(cur_end + INSTRUCTION_SIZE);
                cur_end = regions.last().unwrap().rom_end();
            }

            // Trim the region again to get rid of any junk that may have been found after its end
            trim_region(regions.last_mut().unwrap(), rom_bytes);

            // Skip any return addresses that are now part of the region
            while let Some(cur) = return_addrs.get(i) {
                if *cur >= regions.last().unwrap().rom_end() {
                    break;
                }
                i += 1;
            }
        }
    }
    // println!("{:?}", regions);

    regions
}
