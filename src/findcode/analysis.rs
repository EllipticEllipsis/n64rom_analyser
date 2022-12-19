use super::RomRegion;
// use super::
use crate::utils::*;
use crate::INSTRUCTION_SIZE;
//use rabbitizer;

//use enum_map::Enum;
//use enum_map::EnumMap;
use std::collections::HashMap;
// use strum_macros::EnumIter; // 0.17.1
//use num_enum::TryFromPrimitive;

struct RegisterState {
    initialized: bool,
}

impl Default for RegisterState {
    fn default() -> Self {
        RegisterState { initialized: false }
    }
}
pub struct MyInstruction(pub rabbitizer::Instruction);

impl MyInstruction {
    pub fn new(word: u32) -> Self {
        Self(rabbitizer::Instruction::new(word, 0, rabbitizer::InstrCategory::CPU))
    }
    pub fn new_rsp(word: u32) -> Self {
        Self(rabbitizer::Instruction::new(word, 0, rabbitizer::InstrCategory::RSP))
    }

    // Checks if an instruction has the given operand as an input
    pub fn has_operand_input(&self, operand: rabbitizer::OperandType) -> bool {
        let id = self.0.unique_id;

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
        self.0.destination_gpr() == 0
    }
}

// Treat $v0 and $fv0 as an initialized register
// gcc will use these for the first uninitialized variable reference for ints and floats respectively,
// so enabling this option won't reject gcc functions that begin with a reference to an uninitialized local variable.
const WEAK_UNINITIALIZED_CHECK: bool = true;

// Checks if an instruction references an uninitialized register
fn references_uninitialized(
    my_instruction: &MyInstruction,
    gpr_reg_states: &HashMap<rabbitizer::registers::GprO32, RegisterState>,
    fpr_reg_states: &HashMap<rabbitizer::registers::Cop1O32, RegisterState>,
) -> bool {
    // For each operand type, check if the instruction uses that operand as an input and whether the corresponding register is initialized
    if my_instruction.0.reads_rs() {
        let rs = my_instruction.0.get_rs_o32();
        if !gpr_reg_states[&rs].initialized {
            return true;
        }
    }

    if my_instruction.0.reads_rt() {
        let rt = my_instruction.0.get_rt_o32();
        if !gpr_reg_states[&rt].initialized {
            return true;
        }
    }

    if my_instruction.0.reads_rd() {
        let rd = my_instruction.0.get_rd_o32();
        if !gpr_reg_states[&rd].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::cpu_fs) {
        let fs = my_instruction.0.get_fs_o32();
        if !fpr_reg_states[&fs].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::cpu_ft) {
        let ft = my_instruction.0.get_ft_o32();
        if !fpr_reg_states[&ft].initialized {
            return true;
        }
    }

    if my_instruction.has_operand_input(rabbitizer::OperandType::cpu_fd) {
        let fd = my_instruction.0.get_fd_o32();
        if !fpr_reg_states[&fd].initialized {
            return true;
        }
    }

    false
}

// Check if this instruction is (probably) invalid when at the beginning of a region of code
fn is_invalid_start_instruction(
    my_instruction: &MyInstruction,
    gpr_reg_states: &HashMap<rabbitizer::registers::GprO32, RegisterState>,
    fpr_reg_states: &HashMap<rabbitizer::registers::Cop1O32, RegisterState>,
) -> bool {
    let id = my_instruction.0.unique_id;

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
            if my_instruction.0.get_rs_o32() == rabbitizer::registers::GprO32::zero {
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
            if (my_instruction.0.get_rt_o32() == rabbitizer::registers::GprO32::zero)
                && (my_instruction.0.get_sa() != 0)
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
        && my_instruction.0.get_rs_o32() == rabbitizer::registers::GprO32::ra
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
    //let mut gpr_reg_states: EnumMap<rabbitizer::registers::GprO32, RegisterState> = EnumMap::default();
    //let mut fpr_reg_states: EnumMap<rabbitizer::registers::Cop1O32, RegisterState> = EnumMap::default();
    let mut gpr_reg_states: HashMap<rabbitizer::registers::GprO32, RegisterState> = HashMap::new();
    let mut fpr_reg_states: HashMap<rabbitizer::registers::Cop1O32, RegisterState> = HashMap::new();

    gpr_reg_states.insert(rabbitizer::registers::GprO32::zero, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::at, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::v0, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::v1, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::a0, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::a1, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::a2, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::a3, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::t0, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::t1, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::t2, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::t3, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::t4, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::t5, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::t6, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::t7, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::s0, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::s1, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::s2, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::s3, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::s4, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::s5, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::s6, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::s7, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::t8, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::t9, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::k0, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::k1, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::gp, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::sp, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::fp, RegisterState::default());
    gpr_reg_states.insert(rabbitizer::registers::GprO32::ra, RegisterState::default());

    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fv0, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fv0f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fv1, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fv1f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft0, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft0f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft1, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft1f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft2, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft2f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft3, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft3f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fa0, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fa0f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fa1, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fa1f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft4, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft4f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft5, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::ft5f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs0, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs0f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs1, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs1f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs2, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs2f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs3, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs3f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs4, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs4f, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs5, RegisterState::default());
    fpr_reg_states.insert(rabbitizer::registers::Cop1O32::fs5f, RegisterState::default());

    // Zero is always initialized (it's zero)
    gpr_reg_states.get_mut(&rabbitizer::registers::GprO32::zero).unwrap().initialized = true;

    // The stack pointer and return address always initialized
    gpr_reg_states.get_mut(&rabbitizer::registers::GprO32::sp).unwrap().initialized = true;
    gpr_reg_states.get_mut(&rabbitizer::registers::GprO32::ra).unwrap().initialized = true;

    // Treat all arg registers as initialized
    gpr_reg_states.get_mut(&rabbitizer::registers::GprO32::a0).unwrap().initialized = true;
    gpr_reg_states.get_mut(&rabbitizer::registers::GprO32::a1).unwrap().initialized = true;
    gpr_reg_states.get_mut(&rabbitizer::registers::GprO32::a2).unwrap().initialized = true;
    gpr_reg_states.get_mut(&rabbitizer::registers::GprO32::a3).unwrap().initialized = true;

    // Treat $v0 as initialized for gcc if enabled
    if WEAK_UNINITIALIZED_CHECK {
        gpr_reg_states.get_mut(&rabbitizer::registers::GprO32::v0).unwrap().initialized = true;
    }

    // FPRs

    // Treat all arg registers as initialized
    fpr_reg_states.get_mut(&rabbitizer::registers::Cop1O32::fa0).unwrap().initialized = true;
    fpr_reg_states.get_mut(&rabbitizer::registers::Cop1O32::fa0f).unwrap().initialized = true;
    fpr_reg_states.get_mut(&rabbitizer::registers::Cop1O32::fa1).unwrap().initialized = true;
    fpr_reg_states.get_mut(&rabbitizer::registers::Cop1O32::fa1f).unwrap().initialized = true;

    // Treat $fv0 as initialized for gcc if enabled
    if WEAK_UNINITIALIZED_CHECK {
        fpr_reg_states.get_mut(&rabbitizer::registers::Cop1O32::fv0).unwrap().initialized = true;
        fpr_reg_states.get_mut(&rabbitizer::registers::Cop1O32::fv0f).unwrap().initialized = true;
    }

    let mut instr_index = 0;
    for chunk in rom_bytes[region.rom_start()..].chunks_exact(INSTRUCTION_SIZE) {
        let my_instruction = MyInstruction::new(read_be_word(chunk));

        if !is_invalid_start_instruction(&my_instruction, &gpr_reg_states, &fpr_reg_states) {
            break;
        }
        instr_index += 1;
    }

    // println!("instr_index: {}", instr_index);
    return instr_index;
}
