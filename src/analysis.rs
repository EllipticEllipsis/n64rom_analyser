use crate::findcode;
use crate::findcode::RomRegion;
use crate::utils::*;
use crate::INSTRUCTION_SIZE;
use rabbitizer;

use enum_map::Enum;
use enum_map::EnumMap;
// use strum_macros::EnumIter; // 0.17.1
use num_enum::TryFromPrimitive;

struct RegisterState {
    initialized: bool,
}

impl Default for RegisterState {
    fn default() -> Self {
        RegisterState { initialized: false }
    }
}

#[derive(Enum, Clone, Copy, Hash)]
#[allow(non_camel_case_types)]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum MipsGpr {
    zero = 0,
    at = 1,
    v0 = 2,
    v1 = 3,
    a0 = 4,
    a1 = 5,
    a2 = 6,
    a3 = 7,
    t0 = 8,
    t1 = 9,
    t2 = 10,
    t3 = 11,
    t4 = 12,
    t5 = 13,
    t6 = 14,
    t7 = 15,
    s0 = 16,
    s1 = 17,
    s2 = 18,
    s3 = 19,
    s4 = 20,
    s5 = 21,
    s6 = 22,
    s7 = 23,
    t8 = 24,
    t9 = 25,
    k0 = 26,
    k1 = 27,
    gp = 28,
    sp = 29,
    fp = 30,
    ra = 31,
}

#[derive(Enum, Clone, Copy, Hash)]
#[allow(non_camel_case_types)]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum MipsFpr {
    fv0 = 0,
    fv0f = 1,
    fv1 = 2,
    fv1f = 3,
    ft0 = 4,
    ft0f = 5,
    ft1 = 6,
    ft1f = 7,
    ft2 = 8,
    ft2f = 9,
    ft3 = 10,
    ft3f = 11,
    fa0 = 12,
    fa0f = 13,
    fa1 = 14,
    fa1f = 15,
    ft4 = 16,
    ft4f = 17,
    ft5 = 18,
    ft5f = 19,
    fs0 = 20,
    fs0f = 21,
    fs1 = 22,
    fs1f = 23,
    fs2 = 24,
    fs2f = 25,
    fs3 = 26,
    fs3f = 27,
    fs4 = 28,
    fs4f = 29,
    fs5 = 30,
    fs5f = 31,
}

#[derive(Enum, Clone, Copy, Hash)]
#[allow(non_camel_case_types)]
#[derive(Debug, Eq, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum MipsCop0r {
    Index = 0,
    Random = 1,
    EntryLo0 = 2,
    EntryLo1 = 3,
    Context = 4,
    PageMask = 5,
    Wired = 6,
    // Reserved07 = 7,
    BadVaddr = 8,
    Count = 9,
    EntryHi = 10,
    Compare = 11,
    Status = 12,
    Cause = 13,
    EPC = 14,
    PRevID = 15,
    Config = 16,
    LLAddr = 17,
    WatchLo = 18,
    WatchHi = 19,
    XContext = 20,
    // Reserved21 = 21,
    // Reserved22 = 22,
    // Reserved23 = 23,
    // Reserved24 = 24,
    // Reserved25 = 25,
    PErr = 26,
    CacheErr = 27,
    TagLo = 28,
    TagHi = 29,
    ErrorEPC = 30,
    // Reserved31 = 31,
}

pub struct MyInstruction {
    pub instr: rabbitizer::Instruction,
}

impl MyInstruction {
    pub fn instr_get_rs(&self) -> MipsGpr {
        ((self.instr.raw() >> 21) & 0x1F).try_into().unwrap()
    }
    pub fn instr_get_rt(&self) -> MipsGpr {
        ((self.instr.raw() >> 16) & 0x1F).try_into().unwrap()
    }
    pub fn instr_get_rd(&self) -> MipsGpr {
        ((self.instr.raw() >> 11) & 0x1F).try_into().unwrap()
    }

    pub fn instr_get_fs(&self) -> MipsFpr {
        ((self.instr.raw() >> 21) & 0x1F).try_into().unwrap()
    }
    pub fn instr_get_ft(&self) -> MipsFpr {
        ((self.instr.raw() >> 16) & 0x1F).try_into().unwrap()
    }
    pub fn instr_get_fd(&self) -> MipsFpr {
        ((self.instr.raw() >> 11) & 0x1F).try_into().unwrap()
    }

    pub fn instr_get_sa(&self) -> u32 {
        (self.instr.raw() >> 6) & 0x1F
    }
    pub fn instr_get_op(&self) -> u32 {
        (self.instr.raw() >> 16) & 0x1F
    }

    pub fn instr_get_cop0_rd(&self) -> Result<MipsCop0r, u32> {
        let reg_num = (self.instr.raw() >> 11) & 0x1F;
        let maybe_enum = reg_num.try_into();
        if let Ok(reg) = maybe_enum {
            Ok(reg)
        } else {
            Err(reg_num)
        }
    }

    // Checks if an instruction has the given operand as an input
    pub fn has_operand_input(&self, operand: rabbitizer::OperandType) -> bool {
        let id = self.instr.instr_id();

        // If the instruction has the given operand and doesn't modify it, then it's an input
        if self.instr.has_operand_alias(operand) {
            match operand {
                rabbitizer::OperandType::RAB_OPERAND_cpu_rd => return !self.instr.modifies_rd(),
                rabbitizer::OperandType::RAB_OPERAND_cpu_rt => return !self.instr.modifies_rt(),
                rabbitizer::OperandType::RAB_OPERAND_cpu_rs => {
                    // rs is always an input
                    return true;
                }
                rabbitizer::OperandType::RAB_OPERAND_cpu_fd => {
                    // fd is never an input
                    return false;
                }
                rabbitizer::OperandType::RAB_OPERAND_cpu_ft => {
                    // ft is always an input except for lwc1 and ldc1
                    return !matches!(
                        id,
                        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_lwc1
                            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_ldc1
                    );
                }
                rabbitizer::OperandType::RAB_OPERAND_cpu_fs => {
                    // fs is always an input, except for mtc1 and dmtc1
                    return !matches!(
                        id,
                        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_mtc1
                            | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dmtc1
                    );
                }
                _ => return false,
            }
        }
        return false;
    }

    fn has_zero_output(&self) -> bool {
        if self.instr.modifies_rd() {
            let rd = self.instr_get_rd();
            if rd == MipsGpr::zero {
                return true;
            }
        }

        if self.instr.modifies_rt() {
            let rt = self.instr_get_rt();
            if rt == MipsGpr::zero {
                return true;
            }
        }

        false
    }
}

// Treat $v0 and $fv0 as an initialized register
// gcc will use these for the first uninitialized variable reference for ints and floats respectively,
// so enabling this option won't reject gcc functions that begin with a reference to an uninitialized local variable.
const WEAK_UNINITIALIZED_CHECK: bool = true;

// Checks if an instruction references an uninitialized register
fn references_uninitialized(
    my_instruction: &MyInstruction,
    gpr_reg_states: &EnumMap<MipsGpr, RegisterState>,
    fpr_reg_states: &EnumMap<MipsFpr, RegisterState>,
) -> bool {
    // Retrieve all of the possible operand registers

    // For each operand type, check if the instruction uses that operand as an input and whether the corresponding register is initialized
    if my_instruction.has_operand_input(rabbitizer::OperandType::RAB_OPERAND_cpu_rs) {
        let rs = my_instruction.instr_get_rs();
        if !gpr_reg_states[rs].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::RAB_OPERAND_cpu_rt) {
        let rt = my_instruction.instr_get_rt();
        if !gpr_reg_states[rt].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::RAB_OPERAND_cpu_rd) {
        let rd = my_instruction.instr_get_rd();
        if !gpr_reg_states[rd].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::RAB_OPERAND_cpu_fs) {
        let fs = my_instruction.instr_get_fs();
        if !fpr_reg_states[fs].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::RAB_OPERAND_cpu_ft) {
        let ft = my_instruction.instr_get_ft();
        if !fpr_reg_states[ft].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::RAB_OPERAND_cpu_fd) {
        let fd = my_instruction.instr_get_fd();
        if !fpr_reg_states[fd].initialized {
            return true;
        }
    }

    false
}

// Check if this instruction is (probably) invalid when at the beginning of a region of code
fn is_invalid_start_instruction(
    my_instruction: &MyInstruction,
    gpr_reg_states: &EnumMap<MipsGpr, RegisterState>,
    fpr_reg_states: &EnumMap<MipsFpr, RegisterState>,
) -> bool {
    let id = my_instruction.instr.instr_id();

    // Check if this is a valid instruction to begin with
    if !findcode::is_valid(&my_instruction) {
        return true;
    }

    match id {
        // Code probably won't start with a nop (some functions do, but it'll just be one nop that can be recovered later)
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_nop => return true,

        // Code shouldn't jump to $zero
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_jr => {
            if my_instruction.instr_get_rs() == MipsGpr::zero {
                return true;
            }
        }

        // Shifts with $zero as the input and a non-zero shift amount are likely not real code
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_sll
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_srl
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_sra
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsll
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsll32
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsrl
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsrl32
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsra
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsra32 => {
            // println!(
            //     "test {:?} {:?} {:?}\n",
            //     id,
            //     my_instruction.instr_get_rt(),
            //     my_instruction.instr_get_sa()
            // );
            if my_instruction.instr_get_rt() == MipsGpr::zero && my_instruction.instr_get_sa() != 0
            {
                return true;
            }
        }
        // Code probably won't start with mthi or mtlo
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_mthi
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_mtlo => return true,

        // Code shouldn't start with branches based on the cop1 condition flag (it won't have been set yet)
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_bc1t
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_bc1f
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_bc1tl
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_bc1fl => return true,

        // add/sub and addi are good indicators that the bytes aren't actually instructions, since addu/subu and addiu would normally be used
        rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_add
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_addi
        | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_sub => return true,

        _ => {}
    }
    // Code shouldn't output to $zero
    if my_instruction.has_zero_output() {
        return true;
    }

    // Code shouldn't start with a reference to a register that isn't initialized
    if references_uninitialized(&my_instruction, &gpr_reg_states, &fpr_reg_states) {
        return true;
    }

    // Code shouldn't start with an unconditional branch
    if my_instruction.instr.is_unconditional_branch() {
        return true;
    }

    // Code shouldn't start with a linked jump, as it'd need to save the return address first
    if my_instruction.instr.does_link() {
        return true;
    }

    // Code shouldn't start with a store relative to $ra
    if my_instruction.has_operand_input(rabbitizer::OperandType::RAB_OPERAND_cpu_immediate_base)
        && my_instruction.instr_get_rs() == MipsGpr::ra
    {
        return true;
    }

    false
}
// fn is_invalid_start_instruction(
//     my_instruction: &MyInstruction,
//     gpr_reg_states: &EnumMap<MipsGpr, RegisterState>,
//     fpr_reg_states: &EnumMap<MipsFpr, RegisterState>,
// ) -> bool {
//     let id = my_instruction.instr.instr_id();

//     // Code probably won't start with a nop (some functions do, but it'll just be one nop that can be recovered later)
//     if my_instruction.instr.is_nop() {
//         return true;
//     }

//     // Check if this is a valid instruction to begin with
//     if !findcode::is_valid(&my_instruction) {
//         return true;
//     }

//     // Code shouldn't output to $zero
//     if has_zero_output(my_instruction) {
//         return true;
//     }

//     // Code shouldn't start with a reference to a register that isn't initialized
//     if references_uninitialized(&my_instruction, &gpr_reg_states, &fpr_reg_states) {
//         return true;
//     }

//     // Code shouldn't start with an unconditional branch
//     if my_instruction.instr.is_unconditional_branch() {
//         return true;
//     }

//     // Code shouldn't start with a linked jump, as it'd need to save the return address first
//     if my_instruction.instr.does_link() {
//         return true;
//     }

//     // Code shouldn't jump to $zero
//     if id == rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_jr
//         && my_instruction.instr_get_rs() == MipsGpr::zero
//     {
//         return true;
//     }

//     // Shifts with $zero as the input and a non-zero shift amount are likely not real code
//     if matches!(
//         id,
//         rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_sll
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_srl
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_sra
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsll
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsll32
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsrl
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsrl32
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsra
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_dsra32
//     ) {
//         // println!(
//         //     "test {:?} {:?} {:?}\n",
//         //     id,
//         //     my_instruction.instr_get_rt(),
//         //     my_instruction.instr_get_sa()
//         // );
//         if my_instruction.instr_get_rt() == MipsGpr::zero && my_instruction.instr_get_sa() != 0 {
//             return true;
//         }
//     }

//     // Code probably won't start with mthi or mtlo
//     if matches!(
//         id,
//         rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_mthi
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_mtlo
//     ) {
//         return true;
//     }

//     // Code shouldn't start with branches based on the cop1 condition flag (it won't have been set yet)
//     if matches!(
//         id,
//         rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_bc1t
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_bc1f
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_bc1tl
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_bc1fl
//     ) {
//         return true;
//     }

//     // Add and sub are good indicators that the bytes aren't actually instructions, since addu and subu would normally be used
//     if matches!(
//         id,
//         rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_add
//             | rabbitizer::InstrId::RABBITIZER_INSTR_ID_cpu_sub
//     ) {
//         return true;
//     }

//     // Code shouldn't start with a store relative to $ra
//     if my_instruction
//         .instr
//         .has_operand(rabbitizer::OperandType::RABBITIZER_OPERAND_TYPE_IMM_base)
//         && my_instruction.instr_get_rs() == MipsGpr::ra
//     {
//         return true;
//     }
//     false
// }

pub fn count_invalid_start_instructions(region: &RomRegion, rom_bytes: &[u8]) -> usize {
    let mut gpr_reg_states: EnumMap<MipsGpr, RegisterState> = EnumMap::default();
    let mut fpr_reg_states: EnumMap<MipsFpr, RegisterState> = EnumMap::default();

    // Zero is always initialized (it's zero)
    gpr_reg_states[MipsGpr::zero].initialized = true;

    // The stack pointer and return address always initialized
    gpr_reg_states[MipsGpr::sp].initialized = true;
    gpr_reg_states[MipsGpr::ra].initialized = true;

    // Treat all arg registers as initialized
    gpr_reg_states[MipsGpr::a0].initialized = true;
    gpr_reg_states[MipsGpr::a1].initialized = true;
    gpr_reg_states[MipsGpr::a2].initialized = true;
    gpr_reg_states[MipsGpr::a3].initialized = true;

    // Treat $v0 as initialized for gcc if enabled
    if WEAK_UNINITIALIZED_CHECK {
        gpr_reg_states[MipsGpr::v0].initialized = true;
    }
    // FPRs

    // Treat all arg registers as initialized
    fpr_reg_states[MipsFpr::fa0].initialized = true;
    fpr_reg_states[MipsFpr::fa0f].initialized = true;
    fpr_reg_states[MipsFpr::fa1].initialized = true;
    fpr_reg_states[MipsFpr::fa1f].initialized = true;

    // Treat $fv0 as initialized for gcc if enabled
    if WEAK_UNINITIALIZED_CHECK {
        fpr_reg_states[MipsFpr::fv0].initialized = true;
        fpr_reg_states[MipsFpr::fv0f].initialized = true;
    }

    let mut instr_index = 0;
    for chunk in rom_bytes[region.rom_start()..].chunks_exact(INSTRUCTION_SIZE) {
        let my_instruction = MyInstruction {
            instr: rabbitizer::Instruction::new(read_be_word(chunk), 0),
        };

        if !is_invalid_start_instruction(&my_instruction, &gpr_reg_states, &fpr_reg_states) {
            break;
        }
        instr_index += 1;
    }
    return instr_index;
}
