use crate::analysis;
use crate::analysis::MipsGpr;
use crate::analysis::MyInstruction;
use crate::microcode;
use crate::utils::*;
use crate::INSTRUCTION_SIZE;

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

const JR_RA: u32 = 0x03E00008;

// fn is_ret(t: &(usize, &[u8])) -> bool {
//     return u32::from_be_bytes(t.1.try_into().unwrap()) == JR_RA;
// }

// Search a span for any instances of the instruction `jr $ra`
fn find_return_locations(rom_bytes: &[u8]) -> Vec<usize> {
    // let mut locations = Vec::new();

    // for (i, chunk) in rom_bytes.chunks_exact(INSTRUCTION_SIZE).enumerate() {
    //     if u32::from_be_bytes(chunk.try_into().unwrap()) == JR_RA {
    //         locations.push(INSTRUCTION_SIZE * i);
    //     }
    // }

    // let locations = rom_bytes
    //     .chunks_exact(INSTRUCTION_SIZE)
    //     .enumerate()
    //     .filter(is_ret)
    //     .map(|(index, _)| index)
    //     .collect::<Vec<_>>();
    let locations = rom_bytes
        .chunks_exact(INSTRUCTION_SIZE)
        .enumerate()
        .filter(|(_, v)| read_be_word(*v) == JR_RA)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    locations
}

fn is_unused_n64_instruction(id: rabbitizer::InstrId) -> bool {
    matches!(
        id,
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_ll
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_sc
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_lld
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_scd
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_syscall
    )
}

// Check if a given instruction is valid via several metrics
pub fn is_valid(my_instruction: &MyInstruction) -> bool {
    let id = my_instruction.instr.instr_id();

    // Check for instructions with invalid bits or invalid opcodes
    if rabbitizer::Instruction::is_valid(&my_instruction.instr)
        || id == rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_INVALID
    {
        return false;
    }

    let is_store = my_instruction.instr.does_store();
    let is_load = my_instruction.instr.does_load();

    // Check for loads or stores with an offset from $zero
    if (is_store || is_load) && my_instruction.instr_get_rs() == MipsGpr::zero {
        return false;
    }

    // This check is disabled as some compilers can generate load to $zero for a volatile dereference
    // Check for loads to $zero
    // let is_float = my_instruction.instr.is_float();
    // if is_load && !is_float && my_instruction.instr_get_rt() == MipsGpr::zero {
    //     return false;
    // }

    // Check for mtc0 or mfc0 with invalid registers
    if matches!(
        id,
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_mtc0
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_mfc0
    ) && my_instruction.instr_get_cop0_rd().is_err()
    {
        return false;
    }

    // Check for instructions that wouldn't be in an N64 game, despite being valid
    if is_unused_n64_instruction(id) {
        return false;
    }

    // // Check for cache instructions with invalid parameters
    if id == rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_cache {
        let cache_param = my_instruction.instr_get_op();
        let cache_op = cache_param >> 2;
        let cache_type = cache_param & 0x3;

        // Only cache operations 0-6 and cache types 0-1 are valid
        if cache_op > 6 || cache_type > 1 {
            return false;
        }
    }

    // Check for cop2 instructions, which are invalid for the N64's CPU
    if matches!(
        id,
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_lwc2
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_ldc2
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_swc2
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_sdc2
    ) {
        return false;
    }

    // Check for trap instructions
    if my_instruction.instr.is_trap() {
        return false;
    }

    // Check for ctc0 and cfc0, which aren't valid on the N64
    if matches!(
        id,
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_ctc0
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_cfc0
    ) {
        return false;
    }

    // Check for instructions that don't exist on the N64's CPU
    if matches!(id, rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_pref) {
        return false;
    }

    true
}

fn is_valid_bytes(bytes: &[u8]) -> bool {
    let instr = rabbitizer::Instruction::new(u32::from_be_bytes(bytes.try_into().unwrap()), 0);
    let my_instruction = MyInstruction { instr };
    is_valid(&my_instruction)
}

// Searches backwards from the given rom address until it hits an invalid instruction
fn find_code_start(rom_bytes: &[u8], rom_addr: usize) -> usize {
    0x1000
        + INSTRUCTION_SIZE
            * rom_bytes[0x1000..rom_addr]
                .chunks_exact(INSTRUCTION_SIZE)
                .rposition(|v| !is_valid_bytes(v))
                .unwrap_or(0)
}

// Searches forwards from the given rom address until it hits an invalid instruction
fn find_code_end(rom_bytes: &[u8], rom_addr: usize) -> usize {
    rom_addr
        + INSTRUCTION_SIZE
            * rom_bytes[rom_addr..]
                .chunks_exact(INSTRUCTION_SIZE)
                .rposition(|v| !is_valid_bytes(v))
                .unwrap_or(rom_bytes.len())
}

// // Check if a given instruction word is an unconditional non-linking branch (i.e. `b`, `j`, or `jr`)
fn is_unconditional_branch(bytes: &[u8]) -> bool {
    let instr = rabbitizer::Instruction::new(read_be_word(bytes), 0);

    matches!(
        instr.instr_id(),
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_b
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_j
            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_jr
    )
}

// Trims zeroes from the start of a code region and "loose" instructions from the end
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

// Check if a given rom range is valid CPU instructions
fn check_range(start: usize, end: usize, rom_bytes: &[u8]) -> bool {
    rom_bytes[start..end]
        .chunks_exact(INSTRUCTION_SIZE)
        .all(is_valid_bytes)
}

pub fn find_code_regions(rom_bytes: &[u8]) -> Vec<RomRegion> {
    let return_addrs = find_return_locations(rom_bytes);
    let mut regions = Vec::with_capacity(0x400);

    // let mut it = return_addrs.iter();
    let mut i = 0;

    while let Some(cur) = return_addrs.get(i) {
        let region_start = find_code_start(rom_bytes, *cur);
        let region_end = find_code_end(rom_bytes, *cur);
        regions.push(RomRegion::new(region_start, region_end));

        while let Some(cur) = return_addrs.get(i) {
            if *cur >= region_end {
                break;
            }
            i += 1;
        }

        trim_region(regions.last_mut().unwrap(), rom_bytes);

        // If the current region is close enough to the previous region, check if there's valid RSP microcode between the two
        let len = regions.len();
        if len > 1 {
            let last_start = regions.last().unwrap().rom_start();
            let penultimate = regions.get_mut(len - 2).unwrap();
            if last_start - penultimate.rom_end() < microcode::CHECK_THRESHHOLD {
                // Check if there's a range of valid CPU instructions between these two regions
                let mut valid_range = check_range(penultimate.rom_end(), last_start, rom_bytes);

                // If there isn't check for RSP instructions
                if !valid_range {
                    valid_range =
                        microcode::check_range(penultimate.rom_end(), last_start, rom_bytes);
                    // If RSP instructions were found, mark the first region as having RSP instructions
                    penultimate.set_has_rsp(true);
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
            // Keep advancing the region's end until either the stop point is reached or something
            // that isn't a valid RSP instruction is seen
            while regions.last().unwrap().rom_end() < rom_bytes.len()
                && microcode::is_valid(&rom_bytes[regions.last().unwrap().rom_end()..])
            {
                let cur_end = regions.last().unwrap().rom_end();
                regions
                    .last_mut()
                    .unwrap()
                    .set_rom_end(cur_end + INSTRUCTION_SIZE);
            }

            // Trim the region again to get rid of any junk that may have been found after its end
            trim_region(regions.last_mut().unwrap(), rom_bytes);

            // Skip any return addresses that are now part of the region
            while let Some(cur) = return_addrs.get(i) {
                if *cur >= region_end {
                    break;
                }
                i += 1;
            }
        }

        i += 1;
    }

    regions
}
