use rabbitizer;
// use enum_map::EnumMap;
// use strum_macros::EnumIter; // 0.17.1
use super::{INSTRUCTION_SIZE, analysis::new_instruction_rsp};
use crate::utils::*;

pub const CHECK_THRESHHOLD: usize = 0x400 * INSTRUCTION_SIZE;

pub fn is_valid(my_instruction: &rabbitizer::Instruction) -> bool {
    let id = my_instruction.unique_id;

    // Check for instructions with invalid opcodes
    if id == rabbitizer::InstrId::rsp_INVALID {
        return false;
    }

    // Check for instructions with invalid bits
    if !my_instruction.is_valid() {
        // ?
        // Make sure this isn't a special jr with
        return false;
    }

    // Check for arithmetic that outputs to $zero
    if my_instruction.outputs_to_gpr_zero() {
        return false;
    }

    match id {
        // Check for mtc0 or mfc0 with invalid registers
        rabbitizer::InstrId::rsp_mtc0 | rabbitizer::InstrId::rsp_mfc0 => {
            if my_instruction.get_cop0d_cop0().descriptor().is_reserved() {
                return false;
            }
        }

        _ => (),
    }
    true
}

pub fn is_valid_bytes(bytes: &[u8]) -> bool {
    let my_instruction = new_instruction_rsp(read_be_word(bytes));
    is_valid(&my_instruction)
}

pub fn check_range(start: usize, end: usize, rom_bytes: &[u8]) -> bool {
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

        let instr = new_instruction_rsp(read_be_word(chunk));
        // See check_range_cpu() for an explanation of this logic.
        if (identical_count >= 3) && instr.does_dereference() {
            return false;
        }
        if !is_valid_bytes(chunk) {
            return false;
        }
    }
    true
}
