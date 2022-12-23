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

pub fn new_instruction_cpu(word: u32) -> rabbitizer::Instruction {
    rabbitizer::Instruction::new(word, 0, rabbitizer::InstrCategory::CPU)
}
pub fn new_instruction_rsp(word: u32) -> rabbitizer::Instruction {
    rabbitizer::Instruction::new(word, 0, rabbitizer::InstrCategory::RSP)
}


// Treat $v0 and $fv0 as an initialized register
// gcc will use these for the first uninitialized variable reference for ints and floats respectively,
// so enabling this option won't reject gcc functions that begin with a reference to an uninitialized local variable.
const WEAK_UNINITIALIZED_CHECK: bool = true;

// Checks if an instruction references an uninitialized register
fn references_uninitialized(
    my_instruction: &rabbitizer::Instruction,
    gpr_reg_states: &HashMap<rabbitizer::registers::GprO32, RegisterState>,
    fpr_reg_states: &HashMap<rabbitizer::registers::Cop1O32, RegisterState>,
) -> bool {
    // For each operand type, check if the instruction uses that operand as an input and whether the corresponding register is initialized
    if my_instruction.reads_rs() {
        let rs = my_instruction.get_rs_o32();
        if !gpr_reg_states[&rs].initialized {
            return true;
        }
    }

    if my_instruction.reads_rt() {
        let rt = my_instruction.get_rt_o32();
        if !gpr_reg_states[&rt].initialized {
            return true;
        }
    }

    if my_instruction.reads_rd() {
        let rd = my_instruction.get_rd_o32();
        if !gpr_reg_states[&rd].initialized {
            return true;
        }
    }

    if my_instruction.reads_fs() {
        let fs = my_instruction.get_fs_o32();
        if !fpr_reg_states[&fs].initialized {
            return true;
        }
    }

    if my_instruction.reads_ft() {
        let ft = my_instruction.get_ft_o32();
        if !fpr_reg_states[&ft].initialized {
            return true;
        }
    }

    if my_instruction.reads_fd() {
        let fd = my_instruction.get_fd_o32();
        if !fpr_reg_states[&fd].initialized {
            return true;
        }
    }

    false
}

// Check if this instruction is (probably) invalid when at the beginning of a region of code
fn is_invalid_start_instruction(
    my_instruction: &rabbitizer::Instruction,
    gpr_reg_states: &HashMap<rabbitizer::registers::GprO32, RegisterState>,
    fpr_reg_states: &HashMap<rabbitizer::registers::Cop1O32, RegisterState>,
) -> bool {
    let id = my_instruction.unique_id;

    // println!("    {}", my_instruction.disassemble(None, 0) );

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
            if my_instruction.get_rs_o32() == rabbitizer::registers::GprO32::zero {
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
            if (my_instruction.get_rt_o32() == rabbitizer::registers::GprO32::zero)
                && (my_instruction.get_sa() != 0)
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
    if my_instruction.outputs_to_gpr_zero() {
        // println!("has zero output");
        return true;
    }

    // Code shouldn't start with an unconditional branch
    if my_instruction.is_unconditional_branch() {
        // println!("unconditional branch");
        return true;
    }

    // Code shouldn't start with a linked jump, as it'd need to save the return address first
    if my_instruction.does_link() {
        // println!("does link");
        return true;
    }

    // Code shouldn't start with a store relative to $ra
    if my_instruction
        .has_operand(rabbitizer::OperandType::cpu_immediate_base)
        && my_instruction.get_rs_o32() == rabbitizer::registers::GprO32::ra
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

pub const GPR_REGISTERS: [rabbitizer::registers::GprO32; 32] = [
    rabbitizer::registers::GprO32::zero,
    rabbitizer::registers::GprO32::at,
    rabbitizer::registers::GprO32::v0,
    rabbitizer::registers::GprO32::v1,
    rabbitizer::registers::GprO32::a0,
    rabbitizer::registers::GprO32::a1,
    rabbitizer::registers::GprO32::a2,
    rabbitizer::registers::GprO32::a3,
    rabbitizer::registers::GprO32::t0,
    rabbitizer::registers::GprO32::t1,
    rabbitizer::registers::GprO32::t2,
    rabbitizer::registers::GprO32::t3,
    rabbitizer::registers::GprO32::t4,
    rabbitizer::registers::GprO32::t5,
    rabbitizer::registers::GprO32::t6,
    rabbitizer::registers::GprO32::t7,
    rabbitizer::registers::GprO32::s0,
    rabbitizer::registers::GprO32::s1,
    rabbitizer::registers::GprO32::s2,
    rabbitizer::registers::GprO32::s3,
    rabbitizer::registers::GprO32::s4,
    rabbitizer::registers::GprO32::s5,
    rabbitizer::registers::GprO32::s6,
    rabbitizer::registers::GprO32::s7,
    rabbitizer::registers::GprO32::t8,
    rabbitizer::registers::GprO32::t9,
    rabbitizer::registers::GprO32::k0,
    rabbitizer::registers::GprO32::k1,
    rabbitizer::registers::GprO32::gp,
    rabbitizer::registers::GprO32::sp,
    rabbitizer::registers::GprO32::fp,
    rabbitizer::registers::GprO32::ra,
];
pub const FPR_REGISTERS: [rabbitizer::registers::Cop1O32; 32] = [
    rabbitizer::registers::Cop1O32::fv0,
    rabbitizer::registers::Cop1O32::fv0f,
    rabbitizer::registers::Cop1O32::fv1,
    rabbitizer::registers::Cop1O32::fv1f,
    rabbitizer::registers::Cop1O32::ft0,
    rabbitizer::registers::Cop1O32::ft0f,
    rabbitizer::registers::Cop1O32::ft1,
    rabbitizer::registers::Cop1O32::ft1f,
    rabbitizer::registers::Cop1O32::ft2,
    rabbitizer::registers::Cop1O32::ft2f,
    rabbitizer::registers::Cop1O32::ft3,
    rabbitizer::registers::Cop1O32::ft3f,
    rabbitizer::registers::Cop1O32::fa0,
    rabbitizer::registers::Cop1O32::fa0f,
    rabbitizer::registers::Cop1O32::fa1,
    rabbitizer::registers::Cop1O32::fa1f,
    rabbitizer::registers::Cop1O32::ft4,
    rabbitizer::registers::Cop1O32::ft4f,
    rabbitizer::registers::Cop1O32::ft5,
    rabbitizer::registers::Cop1O32::ft5f,
    rabbitizer::registers::Cop1O32::fs0,
    rabbitizer::registers::Cop1O32::fs0f,
    rabbitizer::registers::Cop1O32::fs1,
    rabbitizer::registers::Cop1O32::fs1f,
    rabbitizer::registers::Cop1O32::fs2,
    rabbitizer::registers::Cop1O32::fs2f,
    rabbitizer::registers::Cop1O32::fs3,
    rabbitizer::registers::Cop1O32::fs3f,
    rabbitizer::registers::Cop1O32::fs4,
    rabbitizer::registers::Cop1O32::fs4f,
    rabbitizer::registers::Cop1O32::fs5,
    rabbitizer::registers::Cop1O32::fs5f,
];

pub fn count_invalid_start_instructions(region: &RomRegion, rom_bytes: &[u8]) -> usize {
    //let mut gpr_reg_states: EnumMap<rabbitizer::registers::GprO32, RegisterState> = EnumMap::default();
    //let mut fpr_reg_states: EnumMap<rabbitizer::registers::Cop1O32, RegisterState> = EnumMap::default();
    let mut gpr_reg_states: HashMap<rabbitizer::registers::GprO32, RegisterState> = HashMap::new();
    let mut fpr_reg_states: HashMap<rabbitizer::registers::Cop1O32, RegisterState> = HashMap::new();

    for reg in GPR_REGISTERS {
        gpr_reg_states.insert(reg, RegisterState::default());
    }
    for reg in FPR_REGISTERS {
        fpr_reg_states.insert(reg, RegisterState::default());
    }

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
        let my_instruction = new_instruction_cpu(read_be_word(chunk));

        if !is_invalid_start_instruction(&my_instruction, &gpr_reg_states, &fpr_reg_states) {
            break;
        }
        instr_index += 1;
    }

    // println!("instr_index: {}", instr_index);
    return instr_index;
}
