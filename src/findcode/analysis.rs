use super::RomRegion;
// use super::
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

pub struct MyInstruction(pub rabbitizer::Instruction);

impl MyInstruction {
    pub fn new(word: u32) -> Self {
        Self(rabbitizer::Instruction::new(word, 0))
    }
    pub fn new_rsp(word: u32) -> Self {
        Self(rabbitizer::Instruction::new_rsp(word, 0))
    }

    pub fn rs(&self) -> MipsGpr {
        ((self.0.raw() >> 21) & 0x1F).try_into().unwrap()
    }
    pub fn rt(&self) -> MipsGpr {
        ((self.0.raw() >> 16) & 0x1F).try_into().unwrap()
    }
    pub fn rd(&self) -> MipsGpr {
        ((self.0.raw() >> 11) & 0x1F).try_into().unwrap()
    }

    pub fn fs(&self) -> MipsFpr {
        ((self.0.raw() >> 21) & 0x1F).try_into().unwrap()
    }
    pub fn ft(&self) -> MipsFpr {
        ((self.0.raw() >> 16) & 0x1F).try_into().unwrap()
    }
    pub fn fd(&self) -> MipsFpr {
        ((self.0.raw() >> 11) & 0x1F).try_into().unwrap()
    }

    pub fn sa(&self) -> u32 {
        (self.0.raw() >> 6) & 0x1F
    }
    pub fn op(&self) -> u32 {
        (self.0.raw() >> 16) & 0x1F
    }

    pub fn code(&self) -> u32 {
        (self.0.raw() >> 6) & 0xFFFFF
    }
    pub fn code_upper(&self) -> u32 {
        (self.0.raw() >> 16) & 0x3FF
    }
    pub fn code_lower(&self) -> u32 {
        (self.0.raw() >> 6) & 0x3FF
    }

    pub fn instr_get_cop0_rd(&self) -> Result<MipsCop0r, u32> {
        let reg_num = (self.0.raw() >> 11) & 0x1F;
        let maybe_enum = reg_num.try_into();
        maybe_enum.map_err(|_| reg_num)
    }

    // Checks if an instruction has the given operand as an input
    pub fn has_operand_input(&self, operand: rabbitizer::OperandType) -> bool {
        let id = self.0.instr_id();

        // If the instruction has the given operand and doesn't modify it, then it's an input
        if self.0.has_operand_alias(operand) {
            match operand {
                rabbitizer::OperandType::cpu_rd => return !self.0.modifies_rd(),
                rabbitizer::OperandType::cpu_rt => return !self.0.modifies_rt(),
                rabbitizer::OperandType::cpu_rs => {
                    // rs is always an input
                    return true;
                }
                rabbitizer::OperandType::cpu_fd => {
                    // fd is never an input
                    return false;
                }
                rabbitizer::OperandType::cpu_ft => {
                    // ft is always an input except for lwc1 and ldc1
                    return !matches!(
                        id,
                        rabbitizer::InstrId::cpu_lwc1 | rabbitizer::InstrId::cpu_ldc1
                    );
                }
                rabbitizer::OperandType::cpu_fs => {
                    // fs is always an input, except for mtc1 and dmtc1
                    return !matches!(
                        id,
                        rabbitizer::InstrId::cpu_mtc1 | rabbitizer::InstrId::cpu_dmtc1
                    );
                }
                _ => return false,
            }
        }
        return false;
    }

    fn has_zero_output(&self) -> bool {
        if self.0.modifies_rd() {
            let rd = self.rd();
            if rd == MipsGpr::zero {
                return true;
            }
        }

        if self.0.modifies_rt() {
            let rt = self.rt();
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
    // For each operand type, check if the instruction uses that operand as an input and whether the corresponding register is initialized
    if my_instruction.has_operand_input(rabbitizer::OperandType::cpu_rs) {
        let rs = my_instruction.rs();
        if !gpr_reg_states[rs].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::cpu_rt) {
        let rt = my_instruction.rt();
        if !gpr_reg_states[rt].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::cpu_rd) {
        let rd = my_instruction.rd();
        if !gpr_reg_states[rd].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::cpu_fs) {
        let fs = my_instruction.fs();
        if !fpr_reg_states[fs].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::cpu_ft) {
        let ft = my_instruction.ft();
        if !fpr_reg_states[ft].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::cpu_fd) {
        let fd = my_instruction.fd();
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
    let id = my_instruction.0.instr_id();

    // println!("    {}", my_instruction.0.disassemble(None, 0) );

    // Check if this is a valid instruction to begin with
    if !super::is_valid(my_instruction) {
        // println!("Invalid instruction");
        return true;
    }

    match id {
        // Code probably won't start with a nop (some functions do, but it'll just be one nop that can be recovered later)
        rabbitizer::InstrId::cpu_nop => {
            // println!("nop");
            return true;
        }

        // Code shouldn't jump to $zero
        rabbitizer::InstrId::cpu_jr => {
            if my_instruction.rs() == MipsGpr::zero {
                // println!("jump to $zero");
                return true;
            }
        }

        // Shifts with $zero as the input and a non-zero shift amount are likely not real code
        rabbitizer::InstrId::cpu_sll
        | rabbitizer::InstrId::cpu_srl
        | rabbitizer::InstrId::cpu_sra
        | rabbitizer::InstrId::cpu_dsll
        | rabbitizer::InstrId::cpu_dsll32
        | rabbitizer::InstrId::cpu_dsrl
        | rabbitizer::InstrId::cpu_dsrl32
        | rabbitizer::InstrId::cpu_dsra
        | rabbitizer::InstrId::cpu_dsra32 => {
            // println!(
            //     "test {:?} {:?} {:?}\n",
            //     id,
            //     my_instruction.instr_get_rt(),
            //     my_instruction.instr_get_sa()
            // );
            if (my_instruction.rt() == MipsGpr::zero)
                && (my_instruction.sa() != 0)
            {
                // println!("Shift with $zero as input and non-zero sa");
                return true;
            }
        }
        // Code probably won't start with mthi or mtlo
        rabbitizer::InstrId::cpu_mthi | rabbitizer::InstrId::cpu_mtlo => {
            // println!("starts with mthi or mtlo");
            return true;
        }

        // Code shouldn't start with branches based on the cop1 condition flag (it won't have been set yet)
        rabbitizer::InstrId::cpu_bc1t
        | rabbitizer::InstrId::cpu_bc1f
        | rabbitizer::InstrId::cpu_bc1tl
        | rabbitizer::InstrId::cpu_bc1fl => {
            // println!("branch from cop1 condition flag");
            return true;
        }

        // add/sub and addi are good indicators that the bytes aren't actually instructions, since addu/subu and addiu would normally be used
        rabbitizer::InstrId::cpu_add
        | rabbitizer::InstrId::cpu_addi
        | rabbitizer::InstrId::cpu_sub => {
            // println!("add/sub/addi");
            return true;
        }

        _ => {}
    }
    // Code shouldn't output to $zero
    if my_instruction.has_zero_output() {
        // println!("has zero output");
        return true;
    }

    // Code shouldn't start with an unconditional branch
    if my_instruction.0.is_unconditional_branch() {
        // println!("unconditional branch");
        return true;
    }

    // Code shouldn't start with a linked jump, as it'd need to save the return address first
    if my_instruction.0.does_link() {
        // println!("does link");
        return true;
    }

    // Code shouldn't start with a store relative to $ra
    if my_instruction
        .0
        .has_operand(rabbitizer::OperandType::cpu_immediate_base)
        && my_instruction.rs() == MipsGpr::ra
    {
        // println!("store relative to $ra");
        return true;
    }

    // Code shouldn't start with a reference to a register that isn't initialized
    if references_uninitialized(&my_instruction, &gpr_reg_states, &fpr_reg_states) {
        // println!("references uninitialized");
        return true;
    }

    false
}

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
        let my_instruction = MyInstruction(rabbitizer::Instruction::new(read_be_word(chunk), 0));

        if !is_invalid_start_instruction(&my_instruction, &gpr_reg_states, &fpr_reg_states) {
            break;
        }
        instr_index += 1;
    }

    // println!("instr_index: {}", instr_index);
    return instr_index;
}
