use enum_map::Enum;
use rabbitizer;
// use enum_map::EnumMap;
// use strum_macros::EnumIter; // 0.17.1
use super::{
    analysis::{MipsGpr, MyInstruction},
    INSTRUCTION_SIZE,
};
use crate::utils::*;
use num_enum::TryFromPrimitive;

pub const CHECK_THRESHHOLD: usize = 0x400 * INSTRUCTION_SIZE;

#[derive(Enum, Clone, Copy, Hash)]
#[allow(non_camel_case_types)]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum RSPCop0r {
    SP_MEM_ADDR = 0,
    SP_DRAM_ADDR = 1,
    SP_RD_LEN = 2,
    SP_WR_LEN = 3,
    SP_STATUS = 4,
    SP_DMA_FULL = 5,
    SP_DMA_BUSY = 6,
    SP_SEMAPHORE = 7,
    DPC_START = 8,
    DPC_END = 9,
    DPC_CURRENT = 10,
    DPC_STATUS = 11,
    DPC_CLOCK = 12,
    DPC_BUFBUSY = 13,
    DPC_PIPEBUSY = 14,
    DPC_TMEM = 15,
}

pub fn instr_get_cop0_rd(my_instruction: &MyInstruction) -> Result<RSPCop0r, u32> {
    let reg_num = (my_instruction.0.raw() >> 11) & 0x1F;
    let maybe_enum = reg_num.try_into();
    if let Ok(reg) = maybe_enum {
        Ok(reg)
    } else {
        Err(reg_num)
    }
}

pub fn is_valid(my_instruction: &MyInstruction) -> bool {
    let id = my_instruction.0.instr_id();

    // Check for instructions with invalid opcodes
    if id == rabbitizer::InstrId::rsp_INVALID {
        return false;
    }

    // Check for instructions with invalid bits
    if !my_instruction.0.is_valid() {
        // ?
        // Make sure this isn't a special jr with
        return false;
    }

    // Check for arithmetic that outputs to $zero
    if my_instruction.0.modifies_rd() && my_instruction.rd() == MipsGpr::zero {
        return false;
    }
    if my_instruction.0.modifies_rt() && my_instruction.rt() == MipsGpr::zero {
        return false;
    }

    match id {
        // Check for mtc0 or mfc0 with invalid registers
        rabbitizer::InstrId::rsp_mtc0 | rabbitizer::InstrId::rsp_mfc0 => {
            if instr_get_cop0_rd(&my_instruction).is_err() {
                return false;
            }
        }

        // Check for nonexistent RSP instructions
        rabbitizer::InstrId::rsp_lwc1
        | rabbitizer::InstrId::rsp_swc1
        | rabbitizer::InstrId::cpu_ctc0
        | rabbitizer::InstrId::cpu_cfc0
        | rabbitizer::InstrId::rsp_cache => return false,
        _ => (),
    }
    true
}

pub fn is_valid_bytes(bytes: &[u8]) -> bool {
    let my_instruction = MyInstruction::new_rsp(read_be_word(bytes));
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

        let instr = MyInstruction::new_rsp(read_be_word(chunk));
        // See check_range_cpu() for an explanation of this logic.
        if (identical_count >= 3) && (instr.0.does_load() || instr.0.does_store()) {
            return false;
        }
        if !is_valid_bytes(chunk) {
            return false;
        }
    }
    true
}
