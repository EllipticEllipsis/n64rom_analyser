use enum_map::Enum;
use rabbitizer;
// use enum_map::EnumMap;
// use strum_macros::EnumIter; // 0.17.1
use crate::{analysis::MyInstruction, INSTRUCTION_SIZE};
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
    let reg_num = (my_instruction.instr.raw() >> 11) & 0x1F;
    let maybe_enum = reg_num.try_into();
    if let Ok(reg) = maybe_enum {
        Ok(reg)
    } else {
        Err(reg_num)
    }
}

pub fn is_valid(bytes: &[u8]) -> bool {
    let instr = rabbitizer::Instruction::new(u32::from_be_bytes(bytes.try_into().unwrap()), 0);
    let my_instruction = MyInstruction { instr };
    let id = my_instruction.instr.instr_id();

    // Check for instructions with invalid opcodes
    if id == rabbitizer::InstrId::RABBITIZER_INSTR_ID_rsp_INVALID {
        return false;
    }

    // Check for instructions with invalid bits
    if !my_instruction.instr.is_valid() {
        // ?
        // Make sure this isn't a special jr with
        return false;
    }

    // Check for mtc0 or mfc0 with invalid registers
    match id {
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_rsp_mtc0
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_rsp_mfc0 => {
            if instr_get_cop0_rd(&my_instruction).is_err() {
                return false;
            }
        }
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_rsp_lwc1
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_rsp_swc1 => return false,
        _ => return true,
    }
    true
}

pub fn check_range(start: usize, end: usize, rom_bytes: &[u8]) -> bool {
    // TODO: is weird extra check required here?
    rom_bytes[start..end]
        .chunks_exact(INSTRUCTION_SIZE)
        .all(is_valid)
}
