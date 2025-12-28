/// Z80 code generator for bc with arbitrary-precision BCD arithmetic
///
/// BCD Number Format in memory (compact):
/// - Byte 0: Flags (bit 7 = sign: 0=positive, 1=negative)
/// - Byte 1: Total digit count (max 100)
/// - Byte 2: Scale (digits after decimal point)
/// - Byte 3+: Packed BCD digits (2 per byte, high nibble first)
///
/// Maximum precision: 100 digits (50 bytes of BCD data + 3 header = 53 bytes max)
/// Numbers are stored with implicit decimal point based on scale.

use crate::bytecode::{BcNum, CompiledModule, Op};

// Z80 opcodes
#[allow(dead_code)]
mod opcodes {
    pub const NOP: u8 = 0x00;
    pub const LD_BC_NN: u8 = 0x01;
    pub const LD_DE_NN: u8 = 0x11;
    pub const LD_HL_NN: u8 = 0x21;
    pub const LD_SP_NN: u8 = 0x31;
    pub const LD_A_N: u8 = 0x3E;
    pub const LD_B_N: u8 = 0x06;
    pub const LD_C_N: u8 = 0x0E;
    pub const LD_D_N: u8 = 0x16;
    pub const LD_E_N: u8 = 0x1E;
    pub const LD_H_N: u8 = 0x26;
    pub const LD_L_N: u8 = 0x2E;

    pub const LD_A_HL: u8 = 0x7E;
    pub const LD_A_DE: u8 = 0x1A;
    pub const LD_A_BC: u8 = 0x0A;
    pub const LD_HL_A: u8 = 0x77;
    pub const LD_DE_A: u8 = 0x12;
    pub const LD_BC_A: u8 = 0x02;

    pub const LD_A_B: u8 = 0x78;
    pub const LD_A_C: u8 = 0x79;
    pub const LD_A_D: u8 = 0x7A;
    pub const LD_A_E: u8 = 0x7B;
    pub const LD_A_H: u8 = 0x7C;
    pub const LD_A_L: u8 = 0x7D;
    pub const LD_B_A: u8 = 0x47;
    pub const LD_C_A: u8 = 0x4F;
    pub const LD_D_A: u8 = 0x57;
    pub const LD_E_A: u8 = 0x5F;
    pub const LD_H_A: u8 = 0x67;
    pub const LD_L_A: u8 = 0x6F;

    pub const LD_B_HL: u8 = 0x46;
    pub const LD_C_HL: u8 = 0x4E;
    pub const LD_D_HL: u8 = 0x56;
    pub const LD_E_HL: u8 = 0x5E;
    pub const LD_H_HL: u8 = 0x66;
    pub const LD_L_HL: u8 = 0x6E;

    pub const LD_HL_B: u8 = 0x70;
    pub const LD_HL_C: u8 = 0x71;
    pub const LD_HL_D: u8 = 0x72;
    pub const LD_HL_E: u8 = 0x73;

    pub const LD_B_C: u8 = 0x41;
    pub const LD_B_D: u8 = 0x42;
    pub const LD_B_E: u8 = 0x43;
    pub const LD_C_B: u8 = 0x48;
    pub const LD_C_D: u8 = 0x4A;
    pub const LD_C_E: u8 = 0x4B;
    pub const LD_D_B: u8 = 0x50;
    pub const LD_D_C: u8 = 0x51;
    pub const LD_E_B: u8 = 0x58;
    pub const LD_E_C: u8 = 0x59;
    pub const LD_H_B: u8 = 0x60;
    pub const LD_H_D: u8 = 0x62;
    pub const LD_H_E: u8 = 0x63;
    pub const LD_L_B: u8 = 0x68;
    pub const LD_L_D: u8 = 0x6A;
    pub const LD_L_E: u8 = 0x6B;

    pub const INC_HL: u8 = 0x23;
    pub const DEC_HL: u8 = 0x2B;
    pub const INC_DE: u8 = 0x13;
    pub const DEC_DE: u8 = 0x1B;
    pub const INC_BC: u8 = 0x03;
    pub const DEC_BC: u8 = 0x0B;
    pub const INC_A: u8 = 0x3C;
    pub const DEC_A: u8 = 0x3D;
    pub const INC_B: u8 = 0x04;
    pub const DEC_B: u8 = 0x05;
    pub const INC_C: u8 = 0x0C;
    pub const DEC_C: u8 = 0x0D;
    pub const INC_D: u8 = 0x14;
    pub const DEC_D: u8 = 0x15;
    pub const INC_E: u8 = 0x1C;
    pub const DEC_E: u8 = 0x1D;

    pub const ADD_A_A: u8 = 0x87;
    pub const ADD_A_B: u8 = 0x80;
    pub const ADD_A_C: u8 = 0x81;
    pub const ADD_A_D: u8 = 0x82;
    pub const ADD_A_E: u8 = 0x83;
    pub const ADD_A_H: u8 = 0x84;
    pub const ADD_A_L: u8 = 0x85;
    pub const ADD_A_HL: u8 = 0x86;
    pub const ADD_A_N: u8 = 0xC6;

    pub const ADC_A_A: u8 = 0x8F;
    pub const ADC_A_B: u8 = 0x88;
    pub const ADC_A_C: u8 = 0x89;
    pub const ADC_A_D: u8 = 0x8A;
    pub const ADC_A_E: u8 = 0x8B;
    pub const ADC_A_HL: u8 = 0x8E;
    pub const ADC_A_N: u8 = 0xCE;

    pub const SUB_A: u8 = 0x97;
    pub const SUB_B: u8 = 0x90;
    pub const SUB_C: u8 = 0x91;
    pub const SUB_D: u8 = 0x92;
    pub const SUB_E: u8 = 0x93;
    pub const SUB_H: u8 = 0x94;
    pub const SUB_L: u8 = 0x95;
    pub const SUB_HL: u8 = 0x96;
    pub const SUB_N: u8 = 0xD6;

    pub const SBC_A_A: u8 = 0x9F;
    pub const SBC_A_B: u8 = 0x98;
    pub const SBC_A_C: u8 = 0x99;
    pub const SBC_A_D: u8 = 0x9A;
    pub const SBC_A_E: u8 = 0x9B;
    pub const SBC_A_HL: u8 = 0x9E;
    pub const SBC_A_N: u8 = 0xDE;

    pub const AND_A: u8 = 0xA7;
    pub const AND_B: u8 = 0xA0;
    pub const AND_C: u8 = 0xA1;
    pub const AND_HL: u8 = 0xA6;
    pub const AND_N: u8 = 0xE6;

    pub const OR_A: u8 = 0xB7;
    pub const OR_B: u8 = 0xB0;
    pub const OR_C: u8 = 0xB1;
    pub const OR_D: u8 = 0xB2;
    pub const OR_E: u8 = 0xB3;
    pub const OR_H: u8 = 0xB4;
    pub const OR_L: u8 = 0xB5;
    pub const OR_HL: u8 = 0xB6;
    pub const OR_N: u8 = 0xF6;

    pub const XOR_A: u8 = 0xAF;
    pub const XOR_B: u8 = 0xA8;
    pub const XOR_C: u8 = 0xA9;
    pub const XOR_HL: u8 = 0xAE;
    pub const XOR_N: u8 = 0xEE;

    pub const CP_A: u8 = 0xBF;
    pub const CP_B: u8 = 0xB8;
    pub const CP_C: u8 = 0xB9;
    pub const CP_D: u8 = 0xBA;
    pub const CP_E: u8 = 0xBB;
    pub const CP_H: u8 = 0xBC;
    pub const CP_L: u8 = 0xBD;
    pub const CP_HL: u8 = 0xBE;
    pub const CP_N: u8 = 0xFE;

    pub const DAA: u8 = 0x27;
    pub const CPL: u8 = 0x2F;
    pub const NEG: u8 = 0x44; // ED prefix
    pub const SCF: u8 = 0x37;
    pub const CCF: u8 = 0x3F;

    pub const RLCA: u8 = 0x07;
    pub const RRCA: u8 = 0x0F;
    pub const RLA: u8 = 0x17;
    pub const RRA: u8 = 0x1F;

    pub const JP_NN: u8 = 0xC3;
    pub const JP_Z_NN: u8 = 0xCA;
    pub const JP_NZ_NN: u8 = 0xC2;
    pub const JP_C_NN: u8 = 0xDA;
    pub const JP_NC_NN: u8 = 0xD2;
    pub const JP_HL: u8 = 0xE9;

    pub const JR_N: u8 = 0x18;
    pub const JR_Z_N: u8 = 0x28;
    pub const JR_NZ_N: u8 = 0x20;
    pub const JR_C_N: u8 = 0x38;
    pub const JR_NC_N: u8 = 0x30;
    pub const DJNZ_N: u8 = 0x10;

    pub const CALL_NN: u8 = 0xCD;
    pub const CALL_Z_NN: u8 = 0xCC;
    pub const CALL_NZ_NN: u8 = 0xC4;
    pub const CALL_C_NN: u8 = 0xDC;
    pub const CALL_NC_NN: u8 = 0xD4;
    pub const RET: u8 = 0xC9;
    pub const RET_Z: u8 = 0xC8;
    pub const RET_NZ: u8 = 0xC0;
    pub const RET_C: u8 = 0xD8;
    pub const RET_NC: u8 = 0xD0;

    pub const PUSH_AF: u8 = 0xF5;
    pub const PUSH_BC: u8 = 0xC5;
    pub const PUSH_DE: u8 = 0xD5;
    pub const PUSH_HL: u8 = 0xE5;
    pub const POP_AF: u8 = 0xF1;
    pub const POP_BC: u8 = 0xC1;
    pub const POP_DE: u8 = 0xD1;
    pub const POP_HL: u8 = 0xE1;

    pub const EX_DE_HL: u8 = 0xEB;
    pub const EX_SP_HL: u8 = 0xE3;
    pub const EXX: u8 = 0xD9;
    pub const EX_AF_AF: u8 = 0x08;

    pub const LD_NN_HL: u8 = 0x22;
    pub const LD_HL_NN_IND: u8 = 0x2A;
    pub const LD_NN_A: u8 = 0x32;
    pub const LD_A_NN_IND: u8 = 0x3A;

    pub const ADD_HL_BC: u8 = 0x09;
    pub const ADD_HL_DE: u8 = 0x19;
    pub const ADD_HL_HL: u8 = 0x29;
    pub const ADD_HL_SP: u8 = 0x39;

    pub const HALT: u8 = 0x76;
    pub const DI: u8 = 0xF3;
    pub const EI: u8 = 0xFB;

    pub const OUT_N_A: u8 = 0xD3;
    pub const IN_A_N: u8 = 0xDB;

    pub const ED_PREFIX: u8 = 0xED;
    pub const LDIR: u8 = 0xB0;
    pub const LDDR: u8 = 0xB8;
    pub const CPIR: u8 = 0xB1;
    pub const SBC_HL_BC: u8 = 0x42;
    pub const SBC_HL_DE: u8 = 0x52;
    pub const ADC_HL_BC: u8 = 0x4A;
    pub const ADC_HL_DE: u8 = 0x5A;
    pub const LD_NN_BC: u8 = 0x43;
    pub const LD_NN_DE: u8 = 0x53;
    pub const LD_BC_NN_IND: u8 = 0x4B;
    pub const LD_DE_NN_IND: u8 = 0x5B;
}

use opcodes::*;

/// Memory layout
/// Note: Emulator has 8KB protected ROM at 0x0000-0x1FFF
/// RAM starts at 0x8000, stack grows down from 0xFFFF
const RUNTIME_SIZE: u16 = 0x2000;     // 8KB for runtime (to avoid protected area)
const BYTECODE_ORG: u16 = 0x2000;     // Bytecode starts after protected ROM
const STACK_TOP: u16 = 0xFFFF;        // Z80 hardware stack

// VM state in RAM at 0x8000+
const VM_STATE_BASE: u16 = 0x8000;
const VM_PC: u16 = VM_STATE_BASE;           // VM program counter (2 bytes)
const VM_SP: u16 = VM_STATE_BASE + 2;       // VM value stack pointer (2 bytes)
const VM_SCALE: u16 = VM_STATE_BASE + 4;    // Current scale (1 byte)
const VM_IBASE: u16 = VM_STATE_BASE + 5;    // Input base (1 byte)
const VM_OBASE: u16 = VM_STATE_BASE + 6;    // Output base (1 byte)
const VM_HEAP: u16 = VM_STATE_BASE + 8;     // Heap pointer (2 bytes)
const VM_TEMP: u16 = VM_STATE_BASE + 10;    // Temp pointer (2 bytes)

// Pre-allocated constants in RAM
const CONST_ZERO: u16 = VM_STATE_BASE + 0x10;  // Zero constant
const CONST_ONE: u16 = VM_STATE_BASE + 0x18;   // One constant

// Variable storage (26 vars * 2 bytes = 52 bytes for pointers)
const VARS_BASE: u16 = VM_STATE_BASE + 0x20;

// Value stack (pointers to numbers, 64 entries * 2 bytes = 128 bytes)
const VSTACK_BASE: u16 = VM_STATE_BASE + 0x60;
const VSTACK_SIZE: u16 = 128;

// Heap for BCD numbers starts after value stack
const HEAP_START: u16 = VM_STATE_BASE + 0xE0;

// Number format constants
const NUM_HEADER_SIZE: u8 = 3;        // sign + len + scale
const MAX_DIGITS: u8 = 100;           // Max digits per number
const MAX_NUM_SIZE: u8 = 53;          // 3 + 50 packed bytes

pub fn generate_rom(module: &CompiledModule) -> Vec<u8> {
    let mut code = Vec::new();

    // Generate Z80 runtime with all opcode handlers
    generate_runtime(&mut code, module);

    // Pad to BYTECODE_ORG
    while code.len() < RUNTIME_SIZE as usize {
        code.push(NOP);
    }

    // Append bytecode
    code.extend(&module.bytecode);

    // Append number constants in packed format, padded to fixed size
    // Each number is padded to MAX_NUM_SIZE bytes for simple indexing
    for num in &module.numbers {
        let packed = num.to_packed();
        code.extend(&packed);
        // Pad to MAX_NUM_SIZE
        for _ in packed.len()..MAX_NUM_SIZE as usize {
            code.push(0);
        }
    }

    // Append strings (length-prefixed)
    for s in &module.strings {
        code.push(s.len() as u8);
        code.extend(s.as_bytes());
    }

    code
}

fn generate_runtime(code: &mut Vec<u8>, module: &CompiledModule) {
    // =====================================================
    // Entry point at 0x0000
    // =====================================================

    // DI - disable interrupts
    code.push(DI);

    // LD SP, STACK_TOP
    code.push(LD_SP_NN);
    emit_u16(code, STACK_TOP);

    // Initialize VM state
    init_vm_state(code);

    // Initialize constants in RAM
    init_constants(code);

    // Jump to main interpreter loop
    code.push(JP_NN);
    let vm_loop_patch = code.len();
    emit_u16(code, 0); // Placeholder

    // =====================================================
    // Subroutines (called from interpreter)
    // =====================================================

    // --- ACIA output routine (address stored for reference) ---
    let acia_out = code.len() as u16;
    emit_acia_out(code);

    // --- ACIA wait for TX ready ---
    let _acia_wait = code.len() as u16;
    emit_acia_wait(code);

    // --- Print BCD number subroutine ---
    let print_num = code.len() as u16;
    emit_print_bcd_number(code, acia_out);

    // --- Print newline ---
    let print_newline = code.len() as u16;
    emit_print_crlf(code, acia_out);

    // --- Allocate number on heap ---
    let alloc_num = code.len() as u16;
    emit_alloc_number(code);

    // --- Copy number ---
    let copy_num = code.len() as u16;
    emit_copy_number(code);

    // --- BCD Add subroutine ---
    let bcd_add_sub = code.len() as u16;
    emit_bcd_add_routine(code);

    // --- BCD Subtract subroutine ---
    let bcd_sub_sub = code.len() as u16;
    emit_bcd_sub_routine(code);

    // --- BCD Multiply subroutine ---
    let bcd_mul_sub = code.len() as u16;
    emit_bcd_mul_routine(code, bcd_add_sub);

    // --- BCD Divide subroutine ---
    let bcd_div_sub = code.len() as u16;
    emit_bcd_div_routine(code, bcd_sub_sub);

    // --- BCD Compare subroutine ---
    let bcd_cmp_sub = code.len() as u16;
    emit_bcd_cmp_routine(code);

    // --- BCD Negate subroutine ---
    let bcd_neg_sub = code.len() as u16;
    emit_bcd_neg_routine(code);

    // --- Push value stack ---
    let push_vstack = code.len() as u16;
    emit_push_vstack(code);

    // --- Pop value stack ---
    let pop_vstack = code.len() as u16;
    emit_pop_vstack(code);

    // =====================================================
    // Main interpreter loop
    // =====================================================
    let vm_loop = code.len() as u16;

    // Patch the initial jump
    code[vm_loop_patch] = (vm_loop & 0xFF) as u8;
    code[vm_loop_patch + 1] = (vm_loop >> 8) as u8;

    // Fetch opcode: LD HL, (VM_PC)
    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_PC);

    // LD A, (HL) - fetch opcode
    code.push(LD_A_HL);

    // INC HL, store back to VM_PC
    code.push(INC_HL);
    code.push(LD_NN_HL);
    emit_u16(code, VM_PC);

    // Save opcode in B for later
    code.push(LD_B_A);

    // =====================================================
    // Opcode dispatch
    // =====================================================

    // HALT (0x00)
    code.push(OR_A);
    let skip_halt = jr_placeholder(code, JR_NZ_N);
    code.push(HALT);
    patch_jr(code, skip_halt);

    // LoadZero (0x10)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::LoadZero as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    // Push pointer to CONST_ZERO
    code.push(LD_HL_NN);
    emit_u16(code, CONST_ZERO);
    code.push(CALL_NN);
    emit_u16(code, push_vstack);
    code.push(JP_NN);
    emit_u16(code, vm_loop);
    patch_jr(code, skip);

    // LoadOne (0x11)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::LoadOne as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    code.push(LD_HL_NN);
    emit_u16(code, CONST_ONE);
    code.push(CALL_NN);
    emit_u16(code, push_vstack);
    code.push(JP_NN);
    emit_u16(code, vm_loop);
    patch_jr(code, skip);

    // LoadNum (0x12) - load from constant table
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::LoadNum as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_load_num_handler(code, module, push_vstack, vm_loop);
    patch_jr(code, skip);

    // LoadVar (0x20)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::LoadVar as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_load_var_handler(code, push_vstack, vm_loop);
    patch_jr(code, skip);

    // StoreVar (0x21)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::StoreVar as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_store_var_handler(code, pop_vstack, vm_loop);
    patch_jr(code, skip);

    // Add (0x30)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Add as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_binary_op_handler(code, pop_vstack, push_vstack, bcd_add_sub, alloc_num, vm_loop);
    patch_jr(code, skip);

    // Sub (0x31)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Sub as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_binary_op_handler(code, pop_vstack, push_vstack, bcd_sub_sub, alloc_num, vm_loop);
    patch_jr(code, skip);

    // Mul (0x32)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Mul as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_binary_op_handler(code, pop_vstack, push_vstack, bcd_mul_sub, alloc_num, vm_loop);
    patch_jr(code, skip);

    // Div (0x33)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Div as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_binary_op_handler(code, pop_vstack, push_vstack, bcd_div_sub, alloc_num, vm_loop);
    patch_jr(code, skip);

    // Neg (0x36)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Neg as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_unary_op_handler(code, pop_vstack, push_vstack, bcd_neg_sub, copy_num, alloc_num, vm_loop);
    patch_jr(code, skip);

    // Eq (0x40) - comparison
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Eq as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_cmp_handler(code, pop_vstack, push_vstack, bcd_cmp_sub, 0, vm_loop); // 0 = equal
    patch_jr(code, skip);

    // Lt (0x42)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Lt as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_cmp_handler(code, pop_vstack, push_vstack, bcd_cmp_sub, 0xFF, vm_loop); // -1 = less
    patch_jr(code, skip);

    // Gt (0x44)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Gt as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_cmp_handler(code, pop_vstack, push_vstack, bcd_cmp_sub, 1, vm_loop); // 1 = greater
    patch_jr(code, skip);

    // Pop (0x02)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Pop as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    code.push(CALL_NN);
    emit_u16(code, pop_vstack);
    code.push(JP_NN);
    emit_u16(code, vm_loop);
    patch_jr(code, skip);

    // Dup (0x03)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Dup as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    // Get top of stack, push it again
    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_SP);
    code.push(DEC_HL);
    code.push(DEC_HL);
    code.push(LD_D_HL);
    code.push(DEC_HL);
    code.push(LD_E_HL);
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(INC_HL);
    // DE = top value, push it
    code.push(EX_DE_HL);
    code.push(CALL_NN);
    emit_u16(code, push_vstack);
    code.push(JP_NN);
    emit_u16(code, vm_loop);
    patch_jr(code, skip);

    // Print (0x90)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Print as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    code.push(CALL_NN);
    emit_u16(code, pop_vstack);
    // HL = pointer to number
    code.push(CALL_NN);
    emit_u16(code, print_num);
    code.push(JP_NN);
    emit_u16(code, vm_loop);
    patch_jr(code, skip);

    // PrintNewline (0x92)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::PrintNewline as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    code.push(CALL_NN);
    emit_u16(code, print_newline);
    code.push(JP_NN);
    emit_u16(code, vm_loop);
    patch_jr(code, skip);

    // Jump (0x60)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Jump as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_jump_handler(code, vm_loop);
    patch_jr(code, skip);

    // JumpIfZero (0x61)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::JumpIfZero as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_jump_if_zero_handler(code, pop_vstack, vm_loop);
    patch_jr(code, skip);

    // JumpIfNotZero (0x62)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::JumpIfNotZero as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    emit_jump_if_not_zero_handler(code, pop_vstack, vm_loop);
    patch_jr(code, skip);

    // StoreScale (0x29) - pop value and store as scale
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::StoreScale as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    // Pop number from stack, get its value, store in VM_SCALE
    code.push(CALL_NN);
    emit_u16(code, pop_vstack);
    // HL = pointer to BCD number, extract first digit
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(LD_A_HL);  // Get first packed byte
    code.push(AND_N);
    code.push(0xF0);     // High nibble
    code.push(RRA);
    code.push(RRA);
    code.push(RRA);
    code.push(RRA);
    code.push(LD_NN_A);
    emit_u16(code, VM_SCALE);
    code.push(JP_NN);
    emit_u16(code, vm_loop);
    patch_jr(code, skip);

    // Nop (0x01) - do nothing
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(Op::Nop as u8);
    let skip = jr_placeholder(code, JR_NZ_N);
    code.push(JP_NN);
    emit_u16(code, vm_loop);
    patch_jr(code, skip);

    // Unknown opcode - just loop (ignoring unknown opcodes)
    code.push(JP_NN);
    emit_u16(code, vm_loop);
}

// =====================================================
// Helper functions
// =====================================================

fn emit_u16(code: &mut Vec<u8>, val: u16) {
    code.push((val & 0xFF) as u8);
    code.push((val >> 8) as u8);
}

fn jr_placeholder(code: &mut Vec<u8>, opcode: u8) -> usize {
    code.push(opcode);
    let pos = code.len();
    code.push(0); // Placeholder
    pos
}

fn patch_jr(code: &mut Vec<u8>, pos: usize) {
    let offset = (code.len() - pos - 1) as i8;
    code[pos] = offset as u8;
}

fn init_vm_state(code: &mut Vec<u8>) {
    // VM_PC = BYTECODE_ORG
    code.push(LD_HL_NN);
    emit_u16(code, BYTECODE_ORG);
    code.push(LD_NN_HL);
    emit_u16(code, VM_PC);

    // VM_SP = VSTACK_BASE
    code.push(LD_HL_NN);
    emit_u16(code, VSTACK_BASE);
    code.push(LD_NN_HL);
    emit_u16(code, VM_SP);

    // VM_SCALE = 0
    code.push(XOR_A);
    code.push(LD_NN_A);
    emit_u16(code, VM_SCALE);

    // VM_IBASE = 10
    code.push(LD_A_N);
    code.push(10);
    code.push(LD_NN_A);
    emit_u16(code, VM_IBASE);

    // VM_OBASE = 10
    code.push(LD_NN_A);
    emit_u16(code, VM_OBASE);

    // VM_HEAP = HEAP_START
    code.push(LD_HL_NN);
    emit_u16(code, HEAP_START);
    code.push(LD_NN_HL);
    emit_u16(code, VM_HEAP);
}

fn init_constants(code: &mut Vec<u8>) {
    // Constants use fixed 50-digit format (25 packed bytes) for proper BCD alignment
    const FIXED_DIGIT_COUNT: u8 = 50;
    const FIXED_PACKED_BYTES: u8 = 25;

    // CONST_ZERO: sign=0, len=50, scale=0, 25 bytes of 0x00
    code.push(LD_HL_NN);
    emit_u16(code, CONST_ZERO);
    code.push(XOR_A);           // A = 0
    code.push(LD_HL_A);         // sign = 0
    code.push(INC_HL);
    code.push(LD_A_N);
    code.push(FIXED_DIGIT_COUNT);
    code.push(LD_HL_A);         // len = 50
    code.push(INC_HL);
    code.push(XOR_A);
    code.push(LD_HL_A);         // scale = 0
    code.push(INC_HL);
    // Write 25 bytes of 0x00
    code.push(LD_B_N);
    code.push(FIXED_PACKED_BYTES);
    code.push(XOR_A);           // A = 0
    let zero_loop = code.len() as u16;
    code.push(LD_HL_A);
    code.push(INC_HL);
    code.push(DJNZ_N);
    let offset = (zero_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);

    // CONST_ONE: sign=0, len=50, scale=0, 24 bytes of 0x00 then 0x01
    code.push(LD_HL_NN);
    emit_u16(code, CONST_ONE);
    code.push(XOR_A);
    code.push(LD_HL_A);         // sign = 0
    code.push(INC_HL);
    code.push(LD_A_N);
    code.push(FIXED_DIGIT_COUNT);
    code.push(LD_HL_A);         // len = 50
    code.push(INC_HL);
    code.push(XOR_A);
    code.push(LD_HL_A);         // scale = 0
    code.push(INC_HL);
    // Write 24 bytes of 0x00
    code.push(LD_B_N);
    code.push(FIXED_PACKED_BYTES - 1);
    code.push(XOR_A);
    let one_loop = code.len() as u16;
    code.push(LD_HL_A);
    code.push(INC_HL);
    code.push(DJNZ_N);
    let offset = (one_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);
    // Write final byte 0x01
    code.push(LD_A_N);
    code.push(0x01);
    code.push(LD_HL_A);
}

// ACIA ports (matching kz80_lisp implementation)
const ACIA_STATUS_PORT: u8 = 0x80;
const ACIA_DATA_PORT: u8 = 0x81;
const ACIA_TX_READY: u8 = 0x02;  // Bit 1 = TX ready

fn emit_acia_wait(code: &mut Vec<u8>) {
    // Wait for ACIA TX ready (bit 1 of status register)
    let loop_start = code.len() as u16;
    code.push(IN_A_N);
    code.push(ACIA_STATUS_PORT);
    code.push(AND_N);
    code.push(ACIA_TX_READY);
    code.push(JR_Z_N);
    let offset = (loop_start as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);
    code.push(RET);
}

fn emit_acia_out(code: &mut Vec<u8>) {
    // Output A to ACIA
    code.push(PUSH_AF);
    // Wait for ready
    let loop_start = code.len() as u16;
    code.push(IN_A_N);
    code.push(ACIA_STATUS_PORT);
    code.push(AND_N);
    code.push(ACIA_TX_READY);
    code.push(JR_Z_N);
    let offset = (loop_start as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);
    code.push(POP_AF);
    code.push(OUT_N_A);
    code.push(ACIA_DATA_PORT);
    code.push(RET);
}

fn emit_print_crlf(code: &mut Vec<u8>, acia_out: u16) {
    code.push(LD_A_N);
    code.push(0x0D); // CR
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    code.push(LD_A_N);
    code.push(0x0A); // LF
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    code.push(RET);
}

fn emit_print_bcd_number(code: &mut Vec<u8>, acia_out: u16) {
    // Input: HL = pointer to BCD number
    // Format: [sign][len][scale][packed digits...]
    // E = 0 initially (flag: have we printed any digit yet?)

    code.push(PUSH_HL);
    code.push(LD_E_N);
    code.push(0);        // E = 0 (haven't printed any digit yet)

    // Check sign
    code.push(LD_A_HL);
    code.push(AND_N);
    code.push(0x80);
    let skip_minus = jr_placeholder(code, JR_Z_N);

    // Print minus
    code.push(LD_A_N);
    code.push(b'-');
    code.push(CALL_NN);
    emit_u16(code, acia_out);

    patch_jr(code, skip_minus);

    code.push(POP_HL);
    code.push(INC_HL);

    // Get length
    code.push(LD_B_HL);  // B = digit count
    code.push(INC_HL);

    // Get scale (for now ignore, print all as integer)
    code.push(LD_C_HL);  // C = scale (unused for now)
    code.push(INC_HL);

    // HL now points to first packed byte
    // B = remaining digit count
    // E = 0 (no digits printed yet)

    // Print digits - loop until B = 0
    let print_loop = code.len() as u16;

    // Check if done
    code.push(LD_A_B);
    code.push(OR_A);
    code.push(RET_Z);  // Done if no more digits

    // Load packed byte, save it in D for later
    code.push(LD_A_HL);
    code.push(LD_D_A);   // D = packed byte (save for low nibble)
    code.push(PUSH_HL);  // Save pointer

    // Get high nibble: A = D >> 4
    code.push(LD_A_D);
    code.push(RRA);
    code.push(RRA);
    code.push(RRA);
    code.push(RRA);
    code.push(AND_N);
    code.push(0x0F);     // A = high digit

    // Skip leading zeros: if A==0 AND E==0 AND B>1, don't print
    code.push(OR_A);     // Is digit 0?
    let not_zero_high = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_E);   // Have we printed anything yet?
    code.push(OR_A);
    let already_printed_high = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_B);   // Is this the last digit?
    code.push(CP_N);
    code.push(1);
    let is_last_high = jr_placeholder(code, JR_Z_N);
    // Skip this digit (it's a leading zero)
    let skip_high = jr_placeholder(code, JR_N);

    patch_jr(code, not_zero_high);
    patch_jr(code, already_printed_high);
    patch_jr(code, is_last_high);

    // Print the high digit
    code.push(LD_A_D);
    code.push(RRA);
    code.push(RRA);
    code.push(RRA);
    code.push(RRA);
    code.push(AND_N);
    code.push(0x0F);
    code.push(ADD_A_N);
    code.push(b'0');
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    code.push(LD_E_N);
    code.push(1);        // E = 1 (we've printed a digit)

    patch_jr(code, skip_high);

    // Decrement digit count
    code.push(DEC_B);

    // Check if we should print low nibble
    code.push(LD_A_B);
    code.push(OR_A);
    let skip_to_next = jr_placeholder(code, JR_Z_N);

    // Get low nibble: A = D & 0x0F
    code.push(LD_A_D);
    code.push(AND_N);
    code.push(0x0F);     // A = low digit

    // Skip leading zeros: if A==0 AND E==0 AND B>1, don't print
    code.push(OR_A);     // Is digit 0?
    let not_zero_low = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_E);   // Have we printed anything yet?
    code.push(OR_A);
    let already_printed_low = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_B);   // Is this the last digit?
    code.push(CP_N);
    code.push(1);
    let is_last_low = jr_placeholder(code, JR_Z_N);
    // Skip this digit (it's a leading zero)
    let skip_low_print = jr_placeholder(code, JR_N);

    patch_jr(code, not_zero_low);
    patch_jr(code, already_printed_low);
    patch_jr(code, is_last_low);

    // Print the low digit
    code.push(LD_A_D);
    code.push(AND_N);
    code.push(0x0F);
    code.push(ADD_A_N);
    code.push(b'0');
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    code.push(LD_E_N);
    code.push(1);        // E = 1 (we've printed a digit)

    patch_jr(code, skip_low_print);

    // Decrement digit count for low nibble
    code.push(DEC_B);

    patch_jr(code, skip_to_next);

    // Advance to next packed byte
    code.push(POP_HL);
    code.push(INC_HL);

    code.push(JP_NN);
    emit_u16(code, print_loop);
}

fn emit_alloc_number(code: &mut Vec<u8>) {
    // Allocate space for a number on heap
    // Returns HL = pointer to new number
    // Advances heap by MAX_NUM_SIZE

    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_HEAP);
    code.push(PUSH_HL);  // Save result

    // Advance heap
    code.push(LD_DE_NN);
    emit_u16(code, MAX_NUM_SIZE as u16);
    code.push(ADD_HL_DE);
    code.push(LD_NN_HL);
    emit_u16(code, VM_HEAP);

    code.push(POP_HL);   // Return allocated address
    code.push(RET);
}

fn emit_copy_number(code: &mut Vec<u8>) {
    // Copy number from DE to HL
    // Both point to BCD number structures

    code.push(PUSH_HL);
    code.push(PUSH_DE);

    // Use LDIR to copy MAX_NUM_SIZE bytes
    code.push(LD_BC_NN);
    emit_u16(code, MAX_NUM_SIZE as u16);
    code.push(EX_DE_HL);  // HL = source, DE = dest
    code.push(ED_PREFIX);
    code.push(LDIR);

    code.push(POP_DE);
    code.push(POP_HL);
    code.push(RET);
}

fn emit_bcd_add_routine(code: &mut Vec<u8>) {
    // BCD Addition: (HL) = (DE) + (HL)
    // Uses DAA for decimal correction
    // Input: DE = first operand, HL = result (copy of second operand)
    // Process RIGHT TO LEFT for proper carry propagation

    code.push(PUSH_HL);
    code.push(PUSH_DE);

    // Skip to END of packed data (header 3 bytes + 24 bytes = offset 27 = last byte)
    // HL += 27, DE += 27
    code.push(LD_BC_NN);
    emit_u16(code, 27);  // 3 header + 24 = point to last packed byte
    code.push(ADD_HL_BC);
    code.push(EX_DE_HL);
    code.push(ADD_HL_BC);
    code.push(EX_DE_HL);

    // For simplicity, add up to 25 packed bytes (50 digits)
    code.push(LD_B_N);
    code.push(25);

    code.push(OR_A);  // Clear carry

    let add_loop = code.len() as u16;

    // Load bytes (process right to left)
    code.push(LD_A_DE);
    code.push(ADC_A_HL);
    code.push(DAA);        // Decimal adjust!
    code.push(LD_HL_A);

    code.push(DEC_HL);
    code.push(DEC_DE);

    code.push(DJNZ_N);
    let offset = (add_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);

    code.push(POP_DE);
    code.push(POP_HL);
    code.push(RET);
}

fn emit_bcd_sub_routine(code: &mut Vec<u8>) {
    // BCD Subtraction: (HL) = (HL) - (DE)
    // HL = result (copy of first operand)
    // DE = second operand
    // Uses DAA for decimal correction after SBC
    // Process RIGHT TO LEFT for proper borrow propagation

    code.push(PUSH_HL);
    code.push(PUSH_DE);

    // Skip to END of packed data (header 3 bytes + 24 bytes = offset 27 = last byte)
    // HL += 27, DE += 27
    code.push(LD_BC_NN);
    emit_u16(code, 27);  // 3 header + 24 = point to last packed byte
    code.push(ADD_HL_BC);
    code.push(EX_DE_HL);
    code.push(ADD_HL_BC);
    code.push(EX_DE_HL);

    code.push(LD_B_N);
    code.push(25);

    code.push(OR_A);  // Clear carry

    let sub_loop = code.len() as u16;

    // a = (HL) - (DE) with borrow
    // Since there's no SBC A,(DE), use EX DE,HL trick
    code.push(EX_DE_HL);     // Now DE=result, HL=second
    code.push(LD_A_DE);      // A = first operand byte
    code.push(SBC_A_HL);     // A = first - second
    code.push(DAA);          // Decimal adjust for subtraction
    code.push(EX_DE_HL);     // Restore: HL=result, DE=second
    code.push(LD_HL_A);      // Store result

    code.push(DEC_HL);
    code.push(DEC_DE);

    code.push(DJNZ_N);
    let offset = (sub_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);

    code.push(POP_DE);
    code.push(POP_HL);
    code.push(RET);
}

fn emit_bcd_mul_routine(code: &mut Vec<u8>, _add_sub: u16) {
    // Simplified BCD Multiplication for single-digit numbers
    // Input: DE = second operand ptr, HL = result ptr (contains first operand data)
    // Output: result in HL
    //
    // With fixed 50-digit format (25 packed bytes), single-digit numbers are stored as:
    // [sign][len=50][scale=0][24 bytes of 0x00][0x0d] where d is the digit
    // The significant digit is at offset 3+24=27 (last packed byte)

    const LAST_BYTE_OFFSET: u8 = 3 + 24;  // 3 header + 24 leading zero bytes

    code.push(PUSH_HL);      // Save result ptr

    // Get first operand's last digit (from result, which is a copy)
    // Add LAST_BYTE_OFFSET to HL to point to the last packed byte
    code.push(LD_BC_NN);
    emit_u16(code, LAST_BYTE_OFFSET as u16);
    code.push(ADD_HL_BC);
    code.push(LD_A_HL);      // A = last packed byte of first operand
    code.push(AND_N);
    code.push(0x0F);         // Low nibble (single digit)
    code.push(LD_B_A);       // B = digit of first operand

    // Get second operand's last digit
    // At this point: B = first operand digit, HL = result+27, DE = second operand ptr
    // Save B and HL, then calculate second operand + 27
    code.push(PUSH_HL);      // Save result+27 position [stack: result+27]
    code.push(PUSH_BC);      // Save B (first operand digit) [stack: BC, result+27]
    code.push(EX_DE_HL);     // HL = second operand ptr, DE = result+27
    code.push(LD_BC_NN);
    emit_u16(code, LAST_BYTE_OFFSET as u16);
    code.push(ADD_HL_BC);    // HL = second operand + 27
    code.push(LD_A_HL);      // A = last packed byte of second operand
    code.push(AND_N);
    code.push(0x0F);         // Low nibble (single digit)
    code.push(LD_C_A);       // C = digit of second operand
    code.push(POP_AF);       // A = saved B (first operand digit), discard F [stack: result+27]
    code.push(LD_B_A);       // B = first operand digit
    code.push(POP_HL);       // HL = result+27 [stack: empty]

    // Multiply B * C using repeated addition with DAA
    code.push(XOR_A);        // A = 0 (accumulator for result)
    code.push(LD_D_A);       // D = 0 (high digit overflow counter)

    // If C = 0, skip multiplication (result is 0)
    code.push(LD_A_C);
    code.push(OR_A);
    let skip_mul = jr_placeholder(code, JR_Z_N);

    // Add B to accumulator C times
    code.push(XOR_A);        // A = 0 (accumulator)
    let mul_loop = code.len() as u16;
    code.push(ADD_A_B);      // A = A + B
    code.push(DAA);          // Decimal adjust
    let no_carry = jr_placeholder(code, JR_NC_N);
    code.push(INC_D);        // Carry -> D
    patch_jr(code, no_carry);
    code.push(DEC_C);
    code.push(LD_E_A);       // Save A temporarily
    code.push(LD_A_C);
    code.push(OR_A);
    code.push(LD_A_E);       // Restore A
    code.push(JR_NZ_N);
    let offset = (mul_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);

    // Now A = BCD result (e.g., 0x12 for twelve), D = overflow (> 99)
    // The result goes in the last 1-2 bytes of the result slot
    // For the fixed format, we keep len=50 and just update the last 1-2 bytes

    patch_jr(code, skip_mul);
    code.push(LD_E_A);       // E = BCD result (low byte)

    // HL is already pointing to the last byte of result
    // Store the result there (single digit result goes in low nibble)
    code.push(LD_A_E);
    code.push(LD_HL_A);      // Store result in last byte

    // If result >= 0x10, we need to update the second-to-last byte too
    code.push(CP_N);
    code.push(0x10);
    let no_high = jr_placeholder(code, JR_C_N);  // Jump if E < 0x10

    // For 2-digit results like 12 (0x12), the byte is already correctly packed
    // But if we have carry in D, we'd need to handle 3+ digits (not implemented)

    patch_jr(code, no_high);

    code.push(POP_HL);       // Restore result ptr
    code.push(RET);
}

fn emit_bcd_div_routine(code: &mut Vec<u8>, _sub_sub: u16) {
    // BCD Division using repeated subtraction
    // Input: DE = second operand (divisor), HL = result ptr (holds dividend copy)
    // Result: quotient in HL
    //
    // With fixed 50-digit format, single-digit numbers are at offset 27 (last packed byte)
    // The low nibble contains the digit

    const LAST_BYTE_OFFSET: u8 = 3 + 24;  // 3 header + 24 leading zero bytes

    code.push(PUSH_HL);      // Save result ptr (dividend copy)
    code.push(PUSH_DE);      // Save divisor ptr

    // Get dividend's last digit (from result)
    code.push(LD_BC_NN);
    emit_u16(code, LAST_BYTE_OFFSET as u16);
    code.push(ADD_HL_BC);
    code.push(LD_A_HL);      // A = last packed byte of dividend
    code.push(AND_N);
    code.push(0x0F);         // Low nibble
    code.push(LD_B_A);       // B = dividend digit

    // Get divisor's last digit
    code.push(POP_HL);       // HL = divisor ptr
    code.push(PUSH_HL);      // Save it again
    code.push(LD_DE_NN);
    emit_u16(code, LAST_BYTE_OFFSET as u16);
    code.push(ADD_HL_DE);
    code.push(LD_A_HL);      // A = last packed byte of divisor
    code.push(AND_N);
    code.push(0x0F);         // Low nibble
    code.push(LD_C_A);       // C = divisor digit

    // Check divisor not zero
    code.push(LD_A_C);
    code.push(OR_A);
    code.push(POP_DE);       // Discard saved divisor ptr
    code.push(POP_HL);       // HL = result ptr
    let not_zero = jr_placeholder(code, JR_NZ_N);
    // Divisor is 0, just return (result stays as dividend copy which is wrong, but avoids infinite loop)
    code.push(RET);

    patch_jr(code, not_zero);

    code.push(PUSH_HL);      // Save result ptr

    // Count subtractions: quotient = 0
    code.push(LD_D_N);
    code.push(0);            // D = quotient

    // While B >= C, subtract and increment quotient
    let div_loop = code.len() as u16;
    code.push(LD_A_B);
    code.push(CP_C);         // Compare dividend with divisor
    let done_div = jr_placeholder(code, JR_C_N);  // If B < C, done

    code.push(LD_A_B);
    code.push(SUB_C);
    code.push(LD_B_A);       // B = B - C (new dividend) - no DAA needed for single digit
    code.push(INC_D);        // quotient++

    code.push(JP_NN);
    emit_u16(code, div_loop);

    patch_jr(code, done_div);

    // Store quotient in result at the last byte
    code.push(POP_HL);       // HL = result
    code.push(LD_BC_NN);
    emit_u16(code, LAST_BYTE_OFFSET as u16);
    code.push(ADD_HL_BC);    // HL = result + 27
    code.push(LD_A_D);       // A = quotient (single digit)
    code.push(LD_HL_A);      // Store in low nibble

    code.push(RET);
}

fn emit_bcd_cmp_routine(code: &mut Vec<u8>) {
    // Compare two BCD numbers
    // Input: DE = first, HL = second
    // Output: A = -1 if DE < HL, 0 if equal, 1 if DE > HL

    // Simplified: compare byte by byte
    code.push(PUSH_HL);
    code.push(PUSH_DE);

    // Skip to first digit (skip 3-byte header)
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(INC_DE);
    code.push(INC_DE);
    code.push(INC_DE);

    code.push(LD_B_N);
    code.push(25);

    let cmp_loop = code.len() as u16;

    code.push(LD_A_DE);
    code.push(CP_HL);
    let not_equal = jr_placeholder(code, JR_NZ_N);

    code.push(INC_HL);
    code.push(INC_DE);
    code.push(DJNZ_N);
    let offset = (cmp_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);

    // Equal
    code.push(XOR_A);
    code.push(POP_DE);
    code.push(POP_HL);
    code.push(RET);

    patch_jr(code, not_equal);
    // A has result of last CP: carry set if DE < HL
    let greater = jr_placeholder(code, JR_NC_N);
    code.push(LD_A_N);
    code.push(0xFF);  // -1
    code.push(POP_DE);
    code.push(POP_HL);
    code.push(RET);

    patch_jr(code, greater);
    code.push(LD_A_N);
    code.push(1);
    code.push(POP_DE);
    code.push(POP_HL);
    code.push(RET);
}

fn emit_bcd_neg_routine(code: &mut Vec<u8>) {
    // Negate a BCD number (flip sign bit)
    // Input: HL = pointer to number

    code.push(LD_A_HL);
    code.push(XOR_N);
    code.push(0x80);  // Flip sign bit
    code.push(LD_HL_A);
    code.push(RET);
}

fn emit_push_vstack(code: &mut Vec<u8>) {
    // Push HL onto value stack
    code.push(PUSH_DE);
    code.push(EX_DE_HL);  // DE = value to push

    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_SP);

    code.push(LD_A_E);
    code.push(LD_HL_A);
    code.push(INC_HL);
    code.push(LD_A_D);
    code.push(LD_HL_A);
    code.push(INC_HL);

    code.push(LD_NN_HL);
    emit_u16(code, VM_SP);

    code.push(POP_DE);
    code.push(RET);
}

fn emit_pop_vstack(code: &mut Vec<u8>) {
    // Pop from value stack into HL
    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_SP);

    code.push(DEC_HL);
    code.push(LD_D_HL);
    code.push(DEC_HL);
    code.push(LD_E_HL);

    code.push(LD_NN_HL);
    emit_u16(code, VM_SP);

    code.push(EX_DE_HL);  // HL = popped value
    code.push(RET);
}

fn emit_load_num_handler(code: &mut Vec<u8>, module: &CompiledModule, push_vstack: u16, vm_loop: u16) {
    // Read 16-bit index from bytecode
    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_PC);
    code.push(LD_E_HL);
    code.push(INC_HL);
    code.push(LD_D_HL);
    code.push(INC_HL);
    code.push(LD_NN_HL);
    emit_u16(code, VM_PC);

    // DE = index, calculate address in constant table
    // Constants start after bytecode at BYTECODE_ORG + bytecode.len()
    // Each constant is padded to MAX_NUM_SIZE (53) bytes
    let nums_base = BYTECODE_ORG + module.bytecode.len() as u16;

    // Multiply index by MAX_NUM_SIZE (53 = 32 + 16 + 4 + 1)
    // Use shifts and adds: index * 53 = index * 64 - index * 8 - index * 2 - index
    // Or simpler: just add MAX_NUM_SIZE times (slow but works for small indices)
    // For efficiency, we'll use: index * 53 = index * 48 + index * 5 = index * (32+16) + index * (4+1)

    // Simpler approach: store index in BC, add MAX_NUM_SIZE to HL in a loop
    // But this is slow for large indices.

    // Let's use: HL = nums_base, then add DE * MAX_NUM_SIZE
    // We can compute DE * 53 by: DE * 32 + DE * 16 + DE * 4 + DE * 1
    // Using shifts: DE << 5 + DE << 4 + DE << 2 + DE

    code.push(LD_HL_NN);
    emit_u16(code, 0);  // HL = 0

    // Compute DE * MAX_NUM_SIZE (53)
    // Step 1: Add DE to HL (DE * 1)
    code.push(ADD_HL_DE);
    code.push(PUSH_HL);  // Save DE * 1

    // Step 2: DE * 4
    code.push(EX_DE_HL);
    code.push(ADD_HL_HL);  // HL = DE * 2
    code.push(ADD_HL_HL);  // HL = DE * 4
    code.push(EX_DE_HL);   // DE = original_index * 4

    code.push(POP_HL);     // HL = original_index * 1
    code.push(ADD_HL_DE);  // HL = index * 5 (1 + 4)
    code.push(PUSH_HL);    // Save index * 5

    // Step 3: DE * 16
    code.push(EX_DE_HL);
    code.push(ADD_HL_HL);  // HL = index * 8
    code.push(ADD_HL_HL);  // HL = index * 16
    code.push(EX_DE_HL);   // DE = index * 16

    // Step 4: index * 16 + index * 32 = index * 48
    code.push(LD_H_D);
    code.push(LD_L_E);     // HL = index * 16
    code.push(ADD_HL_HL);  // HL = index * 32
    code.push(ADD_HL_DE);  // HL = index * 48

    // Step 5: Add index * 5 to get index * 53
    code.push(POP_DE);     // DE = index * 5
    code.push(ADD_HL_DE);  // HL = index * 53

    // Step 6: Add base address
    code.push(LD_DE_NN);
    emit_u16(code, nums_base);
    code.push(ADD_HL_DE);  // HL = nums_base + index * 53

    code.push(CALL_NN);
    emit_u16(code, push_vstack);

    code.push(JP_NN);
    emit_u16(code, vm_loop);
}

fn emit_load_var_handler(code: &mut Vec<u8>, push_vstack: u16, vm_loop: u16) {
    // Read variable index from bytecode
    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_PC);
    code.push(LD_A_HL);
    code.push(INC_HL);
    code.push(LD_NN_HL);
    emit_u16(code, VM_PC);

    // A = var index, get pointer from VARS_BASE + index * 2
    code.push(LD_L_A);
    code.push(LD_H_N);
    code.push(0);
    code.push(ADD_HL_HL);  // HL = index * 2
    code.push(LD_DE_NN);
    emit_u16(code, VARS_BASE);
    code.push(ADD_HL_DE);

    // HL points to variable slot, load pointer
    code.push(LD_E_HL);
    code.push(INC_HL);
    code.push(LD_D_HL);
    code.push(EX_DE_HL);

    // If zero, push zero constant
    code.push(LD_A_H);
    code.push(OR_L);
    let not_zero = jr_placeholder(code, JR_NZ_N);
    code.push(LD_HL_NN);
    emit_u16(code, CONST_ZERO);
    patch_jr(code, not_zero);

    code.push(CALL_NN);
    emit_u16(code, push_vstack);

    code.push(JP_NN);
    emit_u16(code, vm_loop);
}

fn emit_store_var_handler(code: &mut Vec<u8>, pop_vstack: u16, vm_loop: u16) {
    // Pop value
    code.push(CALL_NN);
    emit_u16(code, pop_vstack);
    code.push(PUSH_HL);  // Save value pointer

    // Read variable index
    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_PC);
    code.push(LD_A_HL);
    code.push(INC_HL);
    code.push(LD_NN_HL);
    emit_u16(code, VM_PC);

    // Calculate var slot address
    code.push(LD_L_A);
    code.push(LD_H_N);
    code.push(0);
    code.push(ADD_HL_HL);
    code.push(LD_DE_NN);
    emit_u16(code, VARS_BASE);
    code.push(ADD_HL_DE);

    // Store pointer
    code.push(POP_DE);  // DE = value pointer
    code.push(LD_A_E);
    code.push(LD_HL_A);
    code.push(INC_HL);
    code.push(LD_A_D);
    code.push(LD_HL_A);

    code.push(JP_NN);
    emit_u16(code, vm_loop);
}

fn emit_binary_op_handler(
    code: &mut Vec<u8>,
    pop_vstack: u16,
    push_vstack: u16,
    op_routine: u16,
    alloc_num: u16,
    vm_loop: u16,
) {
    // Pop two operands (last pushed = first popped)
    // For "a + b", bytecode pushes a then b, so we pop b first, then a
    code.push(CALL_NN);
    emit_u16(code, pop_vstack);
    code.push(PUSH_HL);  // Stack: [second operand (b)]

    code.push(CALL_NN);
    emit_u16(code, pop_vstack);
    code.push(PUSH_HL);  // Stack: [first operand (a), second operand (b)]

    // Allocate result number on heap
    code.push(CALL_NN);
    emit_u16(code, alloc_num);
    // HL = result pointer
    code.push(PUSH_HL);  // Stack: [result, first, second]

    // Copy first operand to result (destination for operation)
    // We need to copy header + all digit bytes
    code.push(POP_DE);   // DE = result
    code.push(POP_HL);   // HL = first operand
    code.push(PUSH_DE);  // Save result
    code.push(PUSH_HL);  // Save first operand

    // Copy first operand to result using LDIR (53 bytes max)
    code.push(LD_BC_NN);
    emit_u16(code, MAX_NUM_SIZE as u16);
    code.push(ED_PREFIX);
    code.push(LDIR);     // HL (source) -> DE (dest), BC bytes

    // Now we have: result contains copy of first operand
    // Stack: [first, result, second]
    code.push(POP_HL);   // Discard first (we copied it)
    code.push(POP_HL);   // HL = result
    code.push(PUSH_HL);  // Save result again

    // Get second operand
    code.push(POP_HL);   // HL = result
    code.push(POP_DE);   // DE = second operand
    code.push(PUSH_HL);  // Save result
    code.push(PUSH_DE);  // Save second

    // Call operation: DE = second operand, HL = result (contains first operand data)
    // The operation adds/subtracts second to/from result
    code.push(CALL_NN);
    emit_u16(code, op_routine);

    // Clean up stack and push result
    code.push(POP_DE);   // Discard second operand
    code.push(POP_HL);   // HL = result

    // Push result onto value stack
    code.push(CALL_NN);
    emit_u16(code, push_vstack);

    code.push(JP_NN);
    emit_u16(code, vm_loop);
}

fn emit_unary_op_handler(
    code: &mut Vec<u8>,
    pop_vstack: u16,
    push_vstack: u16,
    op_routine: u16,
    copy_num: u16,
    alloc_num: u16,
    vm_loop: u16,
) {
    // Pop operand
    code.push(CALL_NN);
    emit_u16(code, pop_vstack);
    code.push(PUSH_HL);

    // Allocate result
    code.push(CALL_NN);
    emit_u16(code, alloc_num);
    code.push(EX_DE_HL);  // DE = result
    code.push(POP_HL);    // HL = operand
    code.push(PUSH_DE);   // Save result

    // Copy operand to result
    code.push(CALL_NN);
    emit_u16(code, copy_num);

    // Apply operation to result
    code.push(POP_HL);    // HL = result
    code.push(CALL_NN);
    emit_u16(code, op_routine);

    // Push result
    code.push(CALL_NN);
    emit_u16(code, push_vstack);

    code.push(JP_NN);
    emit_u16(code, vm_loop);
}

fn emit_cmp_handler(
    code: &mut Vec<u8>,
    pop_vstack: u16,
    push_vstack: u16,
    cmp_routine: u16,
    expected: u8,
    vm_loop: u16,
) {
    // Pop two operands
    code.push(CALL_NN);
    emit_u16(code, pop_vstack);
    code.push(PUSH_HL);

    code.push(CALL_NN);
    emit_u16(code, pop_vstack);
    code.push(POP_DE);

    // HL = first, DE = second
    code.push(EX_DE_HL);

    // Compare
    code.push(CALL_NN);
    emit_u16(code, cmp_routine);

    // A = comparison result
    code.push(CP_N);
    code.push(expected);

    // Push 1 if match, 0 otherwise
    let match_case = jr_placeholder(code, JR_Z_N);
    code.push(LD_HL_NN);
    emit_u16(code, CONST_ZERO);
    let done = code.len();
    code.push(JP_NN);
    emit_u16(code, 0); // Placeholder

    patch_jr(code, match_case);
    code.push(LD_HL_NN);
    emit_u16(code, CONST_ONE);

    let here = code.len() as u16;
    code[done + 1] = (here & 0xFF) as u8;
    code[done + 2] = (here >> 8) as u8;

    code.push(CALL_NN);
    emit_u16(code, push_vstack);

    code.push(JP_NN);
    emit_u16(code, vm_loop);
}

fn emit_jump_handler(code: &mut Vec<u8>, vm_loop: u16) {
    // Read 16-bit address and set VM_PC
    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_PC);
    code.push(LD_E_HL);
    code.push(INC_HL);
    code.push(LD_D_HL);

    // DE = jump target (relative to bytecode start)
    code.push(LD_HL_NN);
    emit_u16(code, BYTECODE_ORG);
    code.push(ADD_HL_DE);

    code.push(LD_NN_HL);
    emit_u16(code, VM_PC);

    code.push(JP_NN);
    emit_u16(code, vm_loop);
}

fn emit_jump_if_zero_handler(code: &mut Vec<u8>, pop_vstack: u16, vm_loop: u16) {
    // Pop condition
    code.push(CALL_NN);
    emit_u16(code, pop_vstack);

    // Check if zero (compare first digit byte)
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(LD_A_HL);
    code.push(OR_A);

    let not_zero = jr_placeholder(code, JR_NZ_N);

    // Is zero - do the jump
    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_PC);
    code.push(LD_E_HL);
    code.push(INC_HL);
    code.push(LD_D_HL);
    code.push(LD_HL_NN);
    emit_u16(code, BYTECODE_ORG);
    code.push(ADD_HL_DE);
    code.push(LD_NN_HL);
    emit_u16(code, VM_PC);
    code.push(JP_NN);
    emit_u16(code, vm_loop);

    patch_jr(code, not_zero);

    // Not zero - skip the jump address
    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_PC);
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(LD_NN_HL);
    emit_u16(code, VM_PC);

    code.push(JP_NN);
    emit_u16(code, vm_loop);
}

fn emit_jump_if_not_zero_handler(code: &mut Vec<u8>, pop_vstack: u16, vm_loop: u16) {
    // Pop condition
    code.push(CALL_NN);
    emit_u16(code, pop_vstack);

    // Check if zero
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(LD_A_HL);
    code.push(OR_A);

    let is_zero = jr_placeholder(code, JR_Z_N);

    // Not zero - do the jump
    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_PC);
    code.push(LD_E_HL);
    code.push(INC_HL);
    code.push(LD_D_HL);
    code.push(LD_HL_NN);
    emit_u16(code, BYTECODE_ORG);
    code.push(ADD_HL_DE);
    code.push(LD_NN_HL);
    emit_u16(code, VM_PC);
    code.push(JP_NN);
    emit_u16(code, vm_loop);

    patch_jr(code, is_zero);

    // Is zero - skip the jump address
    code.push(LD_HL_NN_IND);
    emit_u16(code, VM_PC);
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(LD_NN_HL);
    emit_u16(code, VM_PC);

    code.push(JP_NN);
    emit_u16(code, vm_loop);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_runtime() {
        let module = CompiledModule::new();
        let mut code = Vec::new();
        generate_runtime(&mut code, &module);
        assert!(code.len() > 0);
        assert!(code.len() < RUNTIME_SIZE as usize);
        println!("Runtime size: {} bytes", code.len());
    }

    #[test]
    fn test_bcnum_parse() {
        let num = BcNum::parse("123.456");
        assert!(!num.negative);
        assert_eq!(num.integer_digits, vec![1, 2, 3]);
        assert_eq!(num.decimal_digits, vec![4, 5, 6]);
    }

    #[test]
    fn test_bcnum_negative() {
        let num = BcNum::parse("-42");
        assert!(num.negative);
        assert_eq!(num.integer_digits, vec![4, 2]);
    }

    #[test]
    fn test_bcnum_packed() {
        let num = BcNum::parse("12");
        let packed = num.to_packed();
        // Header: sign(0) + len(2) + scale(0) + packed(0x12)
        assert_eq!(packed[0], 0x00);  // positive
        assert_eq!(packed[1], 2);     // 2 total digits
        assert_eq!(packed[2], 0);     // scale = 0 (no decimal digits)
        assert_eq!(packed[3], 0x12);  // packed digits
    }
}
