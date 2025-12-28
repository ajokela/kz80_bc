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
    pub const LD_B_H: u8 = 0x44;
    pub const LD_B_L: u8 = 0x45;
    pub const LD_C_H: u8 = 0x4C;
    pub const LD_C_L: u8 = 0x4D;
    pub const LD_C_B: u8 = 0x48;
    pub const LD_C_D: u8 = 0x4A;
    pub const LD_C_E: u8 = 0x4B;
    pub const LD_D_B: u8 = 0x50;
    pub const LD_D_C: u8 = 0x51;
    pub const LD_D_H: u8 = 0x54;
    pub const LD_E_L: u8 = 0x5D;
    pub const LD_E_B: u8 = 0x58;
    pub const LD_E_C: u8 = 0x59;
    pub const LD_H_B: u8 = 0x60;
    pub const LD_H_D: u8 = 0x62;
    pub const LD_H_E: u8 = 0x63;
    pub const LD_L_B: u8 = 0x68;
    pub const LD_L_C: u8 = 0x69;
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
    pub const XOR_D: u8 = 0xAA;
    pub const XOR_E: u8 = 0xAB;
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

    // ED-prefixed instructions
    pub const ED_PREFIX: u8 = 0xED;
    pub const LDIR_OP: u8 = 0xB0;
    pub const LDDR_OP: u8 = 0xB8;
    pub const CPIR_OP: u8 = 0xB1;
    pub const SBC_HL_BC_OP: u8 = 0x42;
    pub const SBC_HL_DE_OP: u8 = 0x52;
    pub const ADC_HL_BC_OP: u8 = 0x4A;
    pub const ADC_HL_DE_OP: u8 = 0x5A;
    pub const LD_NN_BC_OP: u8 = 0x43;
    pub const LD_NN_DE_OP: u8 = 0x53;
    pub const LD_BC_NN_IND_OP: u8 = 0x4B;
    pub const LD_DE_NN_IND_OP: u8 = 0x5B;

    // IX register instructions (require DD prefix)
    pub const IX_PREFIX: u8 = 0xDD;
    pub const PUSH_IX_OP: u8 = 0xE5;  // DD E5 = PUSH IX
    pub const POP_IX_OP: u8 = 0xE1;   // DD E1 = POP IX
    pub const LD_IX_NN_OP: u8 = 0x21; // DD 21 nn nn = LD IX, nn
    pub const ADD_IX_BC_OP: u8 = 0x09; // DD 09 = ADD IX, BC
    pub const ADD_IX_DE_OP: u8 = 0x19; // DD 19 = ADD IX, DE
    pub const LD_A_IX_D_OP: u8 = 0x7E; // DD 7E d = LD A, (IX+d)
    pub const LD_B_IX_D_OP: u8 = 0x46; // DD 46 d = LD B, (IX+d)
    pub const LD_C_IX_D_OP: u8 = 0x4E; // DD 4E d = LD C, (IX+d)
    pub const LD_D_IX_D_OP: u8 = 0x56; // DD 56 d = LD D, (IX+d)
    pub const LD_E_IX_D_OP: u8 = 0x5E; // DD 5E d = LD E, (IX+d)
    pub const LD_H_IX_D_OP: u8 = 0x66; // DD 66 d = LD H, (IX+d)
    pub const LD_L_IX_D_OP: u8 = 0x6E; // DD 6E d = LD L, (IX+d)
    pub const LD_IX_D_A_OP: u8 = 0x77; // DD 77 d = LD (IX+d), A
    pub const LD_IX_D_B_OP: u8 = 0x70; // DD 70 d = LD (IX+d), B
    pub const LD_IX_D_C_OP: u8 = 0x71; // DD 71 d = LD (IX+d), C
    pub const LD_IX_D_D_OP: u8 = 0x72; // DD 72 d = LD (IX+d), D
    pub const LD_IX_D_E_OP: u8 = 0x73; // DD 73 d = LD (IX+d), E
    pub const INC_IX_OP: u8 = 0x23;   // DD 23 = INC IX
    pub const DEC_IX_OP: u8 = 0x2B;   // DD 2B = DEC IX
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

// Absolute jump helpers for long jumps (>127 bytes)
fn jp_z_placeholder(code: &mut Vec<u8>) -> usize {
    code.push(JP_Z_NN);
    let pos = code.len();
    emit_u16(code, 0);  // Placeholder
    pos
}

fn jp_placeholder(code: &mut Vec<u8>) -> usize {
    code.push(JP_NN);
    let pos = code.len();
    emit_u16(code, 0);  // Placeholder
    pos
}

fn patch_jp(code: &mut Vec<u8>, pos: usize) {
    let addr = code.len() as u16;
    code[pos] = (addr & 0xFF) as u8;
    code[pos + 1] = (addr >> 8) as u8;
}

// IX register helper functions
fn emit_push_ix(code: &mut Vec<u8>) {
    code.push(IX_PREFIX);
    code.push(PUSH_IX_OP);
}

fn emit_pop_ix(code: &mut Vec<u8>) {
    code.push(IX_PREFIX);
    code.push(POP_IX_OP);
}

fn emit_ld_ix_nn(code: &mut Vec<u8>, val: u16) {
    code.push(IX_PREFIX);
    code.push(LD_IX_NN_OP);
    emit_u16(code, val);
}

fn emit_add_ix_bc(code: &mut Vec<u8>) {
    code.push(IX_PREFIX);
    code.push(ADD_IX_BC_OP);
}

fn emit_add_ix_de(code: &mut Vec<u8>) {
    code.push(IX_PREFIX);
    code.push(ADD_IX_DE_OP);
}

fn emit_ld_a_ix_d(code: &mut Vec<u8>, d: i8) {
    code.push(IX_PREFIX);
    code.push(LD_A_IX_D_OP);
    code.push(d as u8);
}

fn emit_ld_l_ix_d(code: &mut Vec<u8>, d: i8) {
    code.push(IX_PREFIX);
    code.push(LD_L_IX_D_OP);
    code.push(d as u8);
}

fn emit_ld_h_ix_d(code: &mut Vec<u8>, d: i8) {
    code.push(IX_PREFIX);
    code.push(LD_H_IX_D_OP);
    code.push(d as u8);
}

fn emit_inc_ix(code: &mut Vec<u8>) {
    code.push(IX_PREFIX);
    code.push(INC_IX_OP);
}

fn emit_dec_ix(code: &mut Vec<u8>) {
    code.push(IX_PREFIX);
    code.push(DEC_IX_OP);
}

// ED-prefixed instruction helpers
fn emit_sbc_hl_de(code: &mut Vec<u8>) {
    code.push(ED_PREFIX);
    code.push(SBC_HL_DE_OP);
}

fn emit_sbc_hl_bc(code: &mut Vec<u8>) {
    code.push(ED_PREFIX);
    code.push(SBC_HL_BC_OP);
}

fn emit_ldir(code: &mut Vec<u8>) {
    code.push(ED_PREFIX);
    code.push(LDIR_OP);
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
const ACIA_RX_READY: u8 = 0x01;  // Bit 0 = RX ready

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
    // C = scale (number of decimal places)

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
    code.push(LD_B_HL);  // B = digit count (50)
    code.push(INC_HL);

    // Get scale for decimal point placement
    code.push(LD_C_HL);  // C = scale (number of decimal places)
    code.push(INC_HL);

    // HL now points to first packed byte
    // B = remaining digit count
    // C = scale (when B == C, print decimal point)
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

    // Skip leading zeros: if A==0 AND E==0 AND B>1 AND B>C (still in integer part), don't print
    code.push(OR_A);     // Is digit 0?
    let not_zero_high = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_E);   // Have we printed anything yet?
    code.push(OR_A);
    let already_printed_high = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_B);   // Is this the last digit?
    code.push(CP_N);
    code.push(1);
    let is_last_high = jr_placeholder(code, JR_Z_N);
    // Also don't skip if we're in the fractional part (B <= C)
    code.push(LD_A_B);
    code.push(CP_C);     // Compare B with C
    let in_fraction_high = jr_placeholder(code, JR_C_N);  // If B < C, we're in fraction
    let eq_scale_high = jr_placeholder(code, JR_Z_N);     // If B == C, we're at decimal point
    // Skip this digit (it's a leading zero in integer part)
    let skip_high = jr_placeholder(code, JR_N);

    patch_jr(code, not_zero_high);
    patch_jr(code, already_printed_high);
    patch_jr(code, is_last_high);
    patch_jr(code, in_fraction_high);
    patch_jr(code, eq_scale_high);

    // Check if we need to print decimal point before this digit
    // If B == C and C > 0 and E == 1, print '.'
    code.push(LD_A_B);
    code.push(CP_C);
    let no_decimal_high = jr_placeholder(code, JR_NZ_N);  // B != C
    code.push(LD_A_C);
    code.push(OR_A);
    let no_scale_high = jr_placeholder(code, JR_Z_N);     // C == 0
    code.push(LD_A_E);
    code.push(OR_A);
    let not_started_high = jr_placeholder(code, JR_Z_N);  // Haven't printed anything
    // Print decimal point
    code.push(LD_A_N);
    code.push(b'.');
    code.push(CALL_NN);
    emit_u16(code, acia_out);

    patch_jr(code, no_decimal_high);
    patch_jr(code, no_scale_high);
    patch_jr(code, not_started_high);

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

    // Skip leading zeros: if A==0 AND E==0 AND B>1 AND B>C, don't print
    code.push(OR_A);     // Is digit 0?
    let not_zero_low = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_E);   // Have we printed anything yet?
    code.push(OR_A);
    let already_printed_low = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_B);   // Is this the last digit?
    code.push(CP_N);
    code.push(1);
    let is_last_low = jr_placeholder(code, JR_Z_N);
    // Also don't skip if we're in the fractional part (B <= C)
    code.push(LD_A_B);
    code.push(CP_C);
    let in_fraction_low = jr_placeholder(code, JR_C_N);
    let eq_scale_low = jr_placeholder(code, JR_Z_N);
    // Skip this digit (it's a leading zero in integer part)
    let skip_low_print = jr_placeholder(code, JR_N);

    patch_jr(code, not_zero_low);
    patch_jr(code, already_printed_low);
    patch_jr(code, is_last_low);
    patch_jr(code, in_fraction_low);
    patch_jr(code, eq_scale_low);

    // Check if we need to print decimal point before this digit
    code.push(LD_A_B);
    code.push(CP_C);
    let no_decimal_low = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_C);
    code.push(OR_A);
    let no_scale_low = jr_placeholder(code, JR_Z_N);
    code.push(LD_A_E);
    code.push(OR_A);
    let not_started_low = jr_placeholder(code, JR_Z_N);
    // Print decimal point
    code.push(LD_A_N);
    code.push(b'.');
    code.push(CALL_NN);
    emit_u16(code, acia_out);

    patch_jr(code, no_decimal_low);
    patch_jr(code, no_scale_low);
    patch_jr(code, not_started_low);

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
    emit_ldir(code);

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

fn emit_bcd_mul_routine(code: &mut Vec<u8>, bcd_add: u16) {
    // BCD Multiplication using repeated addition
    // Input: DE = multiplier ptr, HL = result ptr (contains multiplicand copy)
    // Output: result in HL
    //
    // Algorithm: result = 0; loop multiplier times: result += multiplicand
    // Uses REPL_TEMP (0x8700) to save multiplicand
    // Supports multipliers 0-9999 (4 BCD digits)

    // Save result ptr and multiplier ptr
    code.push(PUSH_HL);          // [stack: result]
    code.push(PUSH_DE);          // [stack: multiplier, result]

    // Copy multiplicand (from HL) to REPL_TEMP
    code.push(LD_DE_NN);
    emit_u16(code, REPL_TEMP);
    code.push(LD_BC_NN);
    emit_u16(code, 28);
    emit_ldir(code);             // Copy multiplicand to REPL_TEMP

    // Get multiplier value from last 2 packed bytes (up to 4 BCD digits = 0-9999)
    code.push(POP_HL);           // HL = multiplier ptr
    code.push(LD_BC_NN);
    emit_u16(code, 26);
    code.push(ADD_HL_BC);        // HL = multiplier + 26 (byte 26)

    // Read byte 26 (high 2 digits) and byte 27 (low 2 digits)
    code.push(LD_D_HL);          // D = byte 26 (packed BCD)
    code.push(INC_HL);
    code.push(LD_E_HL);          // E = byte 27 (packed BCD)
    // Save these for later
    code.push(PUSH_DE);          // [stack: packed bytes, result]

    // Convert E (byte 27, low 2 digits) to binary (0-99)
    code.push(LD_A_E);
    code.push(LD_B_A);           // B = save packed byte
    code.push(AND_N);
    code.push(0x0F);             // A = low digit
    code.push(LD_C_A);           // C = low digit
    code.push(LD_A_B);           // A = packed byte
    code.push(RRCA);
    code.push(RRCA);
    code.push(RRCA);
    code.push(RRCA);
    code.push(AND_N);
    code.push(0x0F);             // A = high digit
    code.push(LD_B_A);           // B = high digit
    code.push(ADD_A_A);          // A = 2 * high
    code.push(ADD_A_A);          // A = 4 * high
    code.push(ADD_A_B);          // A = 5 * high
    code.push(ADD_A_A);          // A = 10 * high
    code.push(ADD_A_C);          // A = 10 * high + low (0-99)
    code.push(LD_E_A);           // E = byte27 binary value

    // Convert D (byte 26, high 2 digits) to binary (0-99)
    code.push(POP_HL);           // H = byte26, L = byte27 (packed)
    code.push(PUSH_DE);          // Save E (low value) [stack: E, result]
    code.push(LD_A_H);
    code.push(LD_B_A);           // B = save packed byte
    code.push(AND_N);
    code.push(0x0F);             // A = low digit
    code.push(LD_C_A);           // C = low digit
    code.push(LD_A_B);           // A = packed byte
    code.push(RRCA);
    code.push(RRCA);
    code.push(RRCA);
    code.push(RRCA);
    code.push(AND_N);
    code.push(0x0F);             // A = high digit
    code.push(LD_B_A);           // B = high digit
    code.push(ADD_A_A);          // A = 2 * high
    code.push(ADD_A_A);          // A = 4 * high
    code.push(ADD_A_B);          // A = 5 * high
    code.push(ADD_A_A);          // A = 10 * high
    code.push(ADD_A_C);          // A = 10 * high + low (0-99)
    // A = byte26 binary value (0-99), need to multiply by 100

    // Compute A * 100: 100 = 64 + 32 + 4
    // Result in HL (16-bit)
    code.push(LD_L_A);
    code.push(LD_H_N);
    code.push(0);                // HL = A (0-99)

    code.push(ADD_HL_HL);        // HL = A * 2
    code.push(ADD_HL_HL);        // HL = A * 4
    code.push(PUSH_HL);          // Save A * 4
    code.push(ADD_HL_HL);        // HL = A * 8
    code.push(ADD_HL_HL);        // HL = A * 16
    code.push(ADD_HL_HL);        // HL = A * 32
    code.push(PUSH_HL);          // Save A * 32
    code.push(ADD_HL_HL);        // HL = A * 64
    code.push(POP_BC);           // BC = A * 32
    code.push(ADD_HL_BC);        // HL = A * 96
    code.push(POP_BC);           // BC = A * 4
    code.push(ADD_HL_BC);        // HL = A * 100

    // Add low byte (E) to get total: HL = high*100 + low
    code.push(POP_DE);           // E = low value [stack: result]
    code.push(LD_D_N);
    code.push(0);                // DE = low value (0-99)
    code.push(ADD_HL_DE);        // HL = total (0-9999)

    // BC = 16-bit loop counter
    code.push(LD_B_H);
    code.push(LD_C_L);

    // Zero the result buffer
    code.push(POP_HL);           // HL = result ptr [stack: empty]
    code.push(PUSH_HL);          // [stack: result]
    code.push(PUSH_BC);          // [stack: counter, result]

    code.push(INC_HL);
    code.push(INC_HL);
    code.push(INC_HL);           // Skip header
    code.push(LD_B_N);
    code.push(25);
    code.push(XOR_A);
    let zero_loop = code.len() as u16;
    code.push(LD_HL_A);
    code.push(INC_HL);
    code.push(DJNZ_N);
    let back = (zero_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back as u8);

    // Set up result header
    code.push(POP_BC);           // BC = counter
    code.push(POP_HL);           // HL = result ptr
    code.push(PUSH_HL);          // [stack: result]
    code.push(XOR_A);
    code.push(LD_HL_A);          // sign = 0
    code.push(INC_HL);
    code.push(LD_A_N);
    code.push(50);
    code.push(LD_HL_A);          // len = 50
    code.push(INC_HL);
    code.push(XOR_A);
    code.push(LD_HL_A);          // scale = 0

    // Check if counter is 0
    code.push(LD_A_B);
    code.push(OR_C);
    let mul_done = jr_placeholder(code, JR_Z_N);

    // Loop: add multiplicand to result BC times (16-bit counter)
    let mul_loop = code.len() as u16;

    code.push(POP_HL);           // HL = result
    code.push(PUSH_HL);
    code.push(PUSH_BC);          // Save counter

    code.push(LD_DE_NN);
    emit_u16(code, REPL_TEMP);
    code.push(CALL_NN);
    emit_u16(code, bcd_add);

    code.push(POP_BC);           // Restore counter

    // Decrement BC (16-bit)
    code.push(DEC_BC);
    code.push(LD_A_B);
    code.push(OR_C);
    code.push(JR_NZ_N);
    let back2 = (mul_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back2 as u8);

    patch_jr(code, mul_done);

    code.push(POP_HL);           // Return result ptr
    code.push(RET);
}

fn emit_bcd_mul10_routine(code: &mut Vec<u8>) {
    // Multiply BCD number by 10 (shift all nibbles left by 1)
    // Input: HL = BCD pointer
    // Output: BCD is multiplied by 10 in place
    // Preserves: HL (restored to point to BCD header)
    use opcodes::*;

    code.push(PUSH_HL);          // Save original HL

    // Skip header (3 bytes) and point to last packed byte
    code.push(LD_BC_NN);
    emit_u16(code, 3 + 24);      // Header + 24 bytes = last packed byte
    code.push(ADD_HL_BC);

    // B = counter (25 bytes), A = carry (initially 0)
    code.push(LD_B_N);
    code.push(25);
    code.push(XOR_A);            // Carry = 0

    // Loop: process each byte from LSB to MSB
    let mul10_loop = code.len() as u16;
    code.push(LD_C_A);           // C = save carry
    code.push(LD_A_HL);          // A = current byte
    code.push(PUSH_AF);          // Save original byte
    // A = (original << 4) & 0xF0
    code.push(RLCA);
    code.push(RLCA);
    code.push(RLCA);
    code.push(RLCA);             // A = rotated left 4
    code.push(AND_N);
    code.push(0xF0);             // Keep only high nibble (was low)
    code.push(OR_C);             // Add carry from previous byte
    code.push(LD_HL_A);          // Store new byte
    code.push(POP_AF);           // Get original byte
    // A = (original >> 4) & 0x0F (carry for next byte)
    code.push(RRCA);
    code.push(RRCA);
    code.push(RRCA);
    code.push(RRCA);
    code.push(AND_N);
    code.push(0x0F);             // Carry = high nibble of original
    code.push(DEC_HL);           // Move to previous byte
    code.push(DJNZ_N);
    let back = (mul10_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back as u8);

    code.push(POP_HL);           // Restore original HL
    code.push(RET);
}

fn emit_bcd_div_routine(code: &mut Vec<u8>, bcd_sub: u16) {
    // BCD Division using repeated subtraction
    // Input: DE = divisor ptr, HL = result ptr (holds dividend copy)
    // Result: quotient in HL
    //
    // Algorithm:
    // 1. Copy dividend (HL) to REPL_TEMP (working copy)
    // 2. quotient = 0 (16-bit binary counter)
    // 3. Loop: subtract divisor from REPL_TEMP
    //    - Check if result went negative (borrow from subtraction)
    //    - If negative, add divisor back and break
    //    - If positive/zero, increment quotient and continue
    // 4. Convert binary quotient to BCD and store in result
    //
    // Uses REPL_TEMP as working dividend, REPL_TEMP2 to save divisor ptr

    // Save pointers
    code.push(PUSH_HL);          // [stack: result (dividend copy)]
    code.push(PUSH_DE);          // [stack: divisor, result]

    // Copy dividend to REPL_TEMP
    code.push(LD_DE_NN);
    emit_u16(code, REPL_TEMP);
    code.push(LD_BC_NN);
    emit_u16(code, 28);
    emit_ldir(code);             // Copy dividend to REPL_TEMP

    // Initialize quotient counter (16-bit) to 0
    // Stack is [divisor, result], BC = 0 (quotient)
    code.push(LD_BC_NN);
    emit_u16(code, 0);           // BC = quotient = 0

    // Division loop: REPL_TEMP -= divisor until negative
    // Invariant at loop start: BC = quotient, stack = [divisor, result]
    let div_loop = code.len() as u16;

    // Get divisor from stack (peek without popping)
    code.push(POP_DE);           // DE = divisor, stack = [result]
    code.push(PUSH_DE);          // stack = [divisor, result]
    code.push(PUSH_BC);          // Save quotient, stack = [quotient, divisor, result]

    // Call bcd_sub: HL = REPL_TEMP (dividend), DE = divisor
    code.push(LD_HL_NN);
    emit_u16(code, REPL_TEMP);
    code.push(CALL_NN);
    emit_u16(code, bcd_sub);     // REPL_TEMP = REPL_TEMP - divisor

    // Check if we went negative by examining if any packed byte is >= 0x99
    // After BCD subtraction with borrow, bytes that underflowed show as 0x99
    code.push(LD_HL_NN);
    emit_u16(code, REPL_TEMP + 3);  // First packed byte (after header)
    code.push(LD_A_HL);
    code.push(CP_N);
    code.push(0x99);
    let done_div = jr_placeholder(code, JR_NC_N);  // If byte >= 0x99, went negative

    // Subtraction was valid, increment quotient and continue
    code.push(POP_BC);           // BC = quotient, stack = [divisor, result]
    code.push(INC_BC);

    // Check if quotient is getting too large (limit to 9999 = 0x270F)
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(0x27);
    let keep_going = jr_placeholder(code, JR_C_N);
    // Quotient overflow, exit
    let overflow = jp_placeholder(code);

    patch_jr(code, keep_going);
    // Continue looping - BC has new quotient, stack = [divisor, result]
    code.push(JP_NN);
    emit_u16(code, div_loop);

    patch_jr(code, done_div);
    // Went negative - restore quotient from stack
    code.push(POP_BC);           // BC = quotient, stack = [divisor, result]

    // Both done_div and overflow converge here
    // At this point: BC = quotient, stack = [divisor, result]
    patch_jp(code, overflow);

    // Convert BC (binary quotient 0-9999) to BCD and store in result
    // BC already has quotient, just clean up stack
    code.push(POP_DE);           // Discard divisor, stack = [result]
    code.push(POP_HL);           // HL = result ptr, stack = []

    // Zero the result first
    code.push(PUSH_HL);
    code.push(PUSH_BC);          // Save quotient
    code.push(INC_HL);
    code.push(INC_HL);
    code.push(INC_HL);           // Skip header
    code.push(LD_B_N);
    code.push(25);
    code.push(XOR_A);
    let zero_loop = code.len() as u16;
    code.push(LD_HL_A);
    code.push(INC_HL);
    code.push(DJNZ_N);
    let back = (zero_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back as u8);

    // Set up header
    code.push(POP_BC);           // Restore quotient
    code.push(POP_HL);           // HL = result
    code.push(PUSH_HL);
    code.push(XOR_A);
    code.push(LD_HL_A);          // sign = 0
    code.push(INC_HL);
    code.push(LD_A_N);
    code.push(50);
    code.push(LD_HL_A);          // len = 50
    code.push(INC_HL);
    code.push(XOR_A);
    code.push(LD_HL_A);          // scale = 0

    // Convert BC (binary 0-9999) to BCD at byte 26-27
    // BC = binary value
    // We need to convert to packed BCD: high byte at offset 26, low byte at offset 27

    // Binary to BCD conversion using repeated division by 10
    // For each digit: divide by 10, remainder is the digit, quotient becomes new dividend
    // Uses 16-bit division to handle quotients > 255

    // For up to 9999, we need 4 digits = 2 packed bytes
    code.push(POP_HL);           // HL = result
    code.push(PUSH_HL);
    code.push(LD_DE_NN);
    emit_u16(code, 27);
    code.push(ADD_HL_DE);        // HL = result + 27 (last packed byte)
    code.push(PUSH_HL);          // Save position [stack: pos, result]

    // We'll extract 4 digits and store in REPL_TEMP area temporarily
    // REPL_TEMP+0 = units, +1 = tens, +2 = hundreds, +3 = thousands

    // === Extract units digit (BC % 10) ===
    // Use DE as 16-bit quotient counter
    code.push(LD_DE_NN);
    emit_u16(code, 0);           // DE = quotient counter = 0

    let units_loop = code.len() as u16;
    // Subtract 10 from BC
    code.push(LD_A_C);
    code.push(SUB_N);
    code.push(10);
    code.push(LD_C_A);
    code.push(LD_A_B);
    code.push(SBC_A_N);
    code.push(0);
    code.push(LD_B_A);
    let units_done = jr_placeholder(code, JR_C_N);  // If BC < 0 (borrow), done
    code.push(INC_DE);           // quotient++ (16-bit)
    code.push(JP_NN);
    emit_u16(code, units_loop);

    patch_jr(code, units_done);
    // BC went negative, add back 10 to get remainder (units digit)
    code.push(LD_A_C);
    code.push(ADD_A_N);
    code.push(10);
    code.push(LD_NN_A);
    emit_u16(code, REPL_TEMP);   // Store units digit at REPL_TEMP+0

    // BC = DE (quotient becomes new dividend)
    code.push(LD_B_D);
    code.push(LD_C_E);

    // === Extract tens digit (BC % 10) ===
    code.push(LD_DE_NN);
    emit_u16(code, 0);           // DE = quotient counter = 0

    let tens_loop = code.len() as u16;
    code.push(LD_A_C);
    code.push(SUB_N);
    code.push(10);
    code.push(LD_C_A);
    code.push(LD_A_B);
    code.push(SBC_A_N);
    code.push(0);
    code.push(LD_B_A);
    let tens_done = jr_placeholder(code, JR_C_N);
    code.push(INC_DE);
    code.push(JP_NN);
    emit_u16(code, tens_loop);

    patch_jr(code, tens_done);
    code.push(LD_A_C);
    code.push(ADD_A_N);
    code.push(10);
    code.push(LD_NN_A);
    emit_u16(code, REPL_TEMP + 1);  // Store tens digit

    code.push(LD_B_D);
    code.push(LD_C_E);           // BC = quotient

    // === Extract hundreds digit (BC % 10) ===
    code.push(LD_DE_NN);
    emit_u16(code, 0);

    let hunds_loop = code.len() as u16;
    code.push(LD_A_C);
    code.push(SUB_N);
    code.push(10);
    code.push(LD_C_A);
    code.push(LD_A_B);
    code.push(SBC_A_N);
    code.push(0);
    code.push(LD_B_A);
    let hunds_done = jr_placeholder(code, JR_C_N);
    code.push(INC_DE);
    code.push(JP_NN);
    emit_u16(code, hunds_loop);

    patch_jr(code, hunds_done);
    code.push(LD_A_C);
    code.push(ADD_A_N);
    code.push(10);
    code.push(LD_NN_A);
    emit_u16(code, REPL_TEMP + 2);  // Store hundreds digit

    // BC = DE (quotient = thousands digit, should be 0-9)
    code.push(LD_A_E);           // A = thousands digit (low byte of quotient)
    code.push(LD_NN_A);
    emit_u16(code, REPL_TEMP + 3);  // Store thousands digit

    // === Pack digits into BCD bytes ===
    // Low byte (offset 27): (tens << 4) | units
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_TEMP + 1);  // A = tens
    code.push(RLCA);
    code.push(RLCA);
    code.push(RLCA);
    code.push(RLCA);             // A = tens << 4
    code.push(LD_B_A);           // B = tens << 4
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_TEMP);   // A = units
    code.push(OR_B);             // A = (tens << 4) | units
    code.push(POP_HL);           // HL = result + 27 [stack: result]
    code.push(LD_HL_A);          // Store low byte

    // High byte (offset 26): (thousands << 4) | hundreds
    code.push(DEC_HL);           // HL = result + 26
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_TEMP + 3);  // A = thousands
    code.push(RLCA);
    code.push(RLCA);
    code.push(RLCA);
    code.push(RLCA);             // A = thousands << 4
    code.push(LD_B_A);           // B = thousands << 4
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_TEMP + 2);  // A = hundreds
    code.push(OR_B);             // A = (thousands << 4) | hundreds
    code.push(LD_HL_A);          // Store high byte

    code.push(POP_HL);           // Return result ptr
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
    emit_ldir(code);     // HL (source) -> DE (dest), BC bytes

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

// =====================================================
// REPL Mode - Standalone interpreter running on Z80
// =====================================================

// REPL memory layout (different from bytecode VM)
const REPL_INPUT_BUF: u16 = 0x8000;      // 256 bytes for input line
const REPL_INPUT_LEN: u16 = 0x80F0;      // Current input length
const REPL_INPUT_POS: u16 = 0x80F1;      // Current parse position
const REPL_TOKEN_BUF: u16 = 0x8100;      // Tokenized input (64 tokens * 4 bytes)
const REPL_TOKEN_CNT: u16 = 0x81FC;      // Token count
const REPL_TOKEN_POS: u16 = 0x81FE;      // Current token position for parsing
const REPL_OP_STACK: u16 = 0x8200;       // Operator stack (64 entries)
const REPL_OP_SP: u16 = 0x82FE;          // Operator stack pointer
const REPL_VAL_STACK: u16 = 0x8300;      // Value stack (pointers to BCD numbers)
const REPL_VAL_SP: u16 = 0x83FE;         // Value stack pointer
const REPL_VARS: u16 = 0x8400;           // 27 slots * 28 bytes (a-z + scale)
const REPL_SCALE_BCD: u16 = 0x8400 + 26 * 28;  // Scale as BCD (slot 26, same format as variables)
const REPL_TEMP: u16 = 0x8700;           // Temp BCD buffer (28 bytes)
const REPL_TEMP2: u16 = 0x871C;          // Second temp buffer
const REPL_SCALE: u16 = 0x8740;          // Scale setting (1 byte)
const REPL_HEAP: u16 = 0x8800;           // Heap start
const REPL_HEAP_PTR: u16 = 0x87FC;       // Current heap pointer

// Token types for REPL
const TOK_EOF: u8 = 0x00;
const TOK_NUMBER: u8 = 0x01;      // Followed by 2-byte pointer to BCD
const TOK_VARIABLE: u8 = 0x02;    // Followed by variable index (0-25)
const TOK_SCALE: u8 = 0x03;       // Special 'scale' variable
const TOK_PLUS: u8 = 0x10;
const TOK_MINUS: u8 = 0x11;
const TOK_STAR: u8 = 0x12;
const TOK_SLASH: u8 = 0x13;
const TOK_PERCENT: u8 = 0x14;
const TOK_CARET: u8 = 0x15;
const TOK_LPAREN: u8 = 0x20;
const TOK_RPAREN: u8 = 0x21;
const TOK_ASSIGN: u8 = 0x30;

/// Generate a standalone REPL ROM that runs entirely on the Z80
pub fn generate_repl_rom() -> Vec<u8> {
    use opcodes::*;

    let mut code = Vec::new();

    // Jump to init
    code.push(JP_NN);
    let init_patch = code.len();
    emit_u16(&mut code, 0);  // Will be patched

    // Pad to 0x0100 to avoid any protected areas
    while code.len() < 0x0100 {
        code.push(NOP);
    }

    // === Subroutines ===

    // ACIA output character (A = char)
    let acia_out = code.len() as u16;
    emit_repl_acia_out(&mut code);

    // ACIA input character (returns char in A)
    let acia_in = code.len() as u16;
    emit_repl_acia_in(&mut code);

    // Print string (HL = null-terminated string)
    let print_str = code.len() as u16;
    emit_repl_print_str(&mut code, acia_out);

    // Print CRLF
    let print_crlf = code.len() as u16;
    emit_repl_print_crlf(&mut code, acia_out);

    // Get line from input (fills REPL_INPUT_BUF)
    let getline = code.len() as u16;
    emit_repl_getline(&mut code, acia_in, acia_out);

    // Allocate BCD number on heap (returns HL = pointer)
    let alloc_num = code.len() as u16;
    emit_repl_alloc_num(&mut code);

    // Parse number from input buffer (returns HL = BCD pointer)
    let parse_num = code.len() as u16;
    emit_repl_parse_num(&mut code, alloc_num);

    // Tokenize input buffer
    let tokenize = code.len() as u16;
    emit_repl_tokenize(&mut code, parse_num);

    // Push value onto value stack
    let val_push = code.len() as u16;
    emit_repl_val_push(&mut code);

    // Pop value from value stack (returns HL = pointer)
    let val_pop = code.len() as u16;
    emit_repl_val_pop(&mut code);

    // Push operator onto operator stack
    let op_push = code.len() as u16;
    emit_repl_op_push(&mut code);

    // Pop operator from operator stack (returns A = operator)
    let op_pop = code.len() as u16;
    emit_repl_op_pop(&mut code);

    // Check if operator stack is empty (Z flag set if empty)
    let op_empty = code.len() as u16;
    emit_repl_op_empty(&mut code);

    // Peek top of operator stack (returns A = operator)
    let op_peek = code.len() as u16;
    emit_repl_op_peek(&mut code);

    // Get operator precedence (A = token, returns A = precedence)
    let get_prec = code.len() as u16;
    emit_repl_get_prec(&mut code);

    // BCD arithmetic routines
    let bcd_add = code.len() as u16;
    emit_bcd_add_routine(&mut code);

    let bcd_sub = code.len() as u16;
    emit_bcd_sub_routine(&mut code);

    let bcd_mul = code.len() as u16;
    emit_bcd_mul_routine(&mut code, bcd_add);

    let bcd_div = code.len() as u16;
    emit_bcd_div_routine(&mut code, bcd_sub);

    // Multiply BCD by 10 (shift digits left)
    let bcd_mul10 = code.len() as u16;
    emit_bcd_mul10_routine(&mut code);

    // Copy BCD number (HL = dest, DE = source) - use REPL 28-byte version
    let bcd_copy = code.len() as u16;
    emit_repl_copy_number(&mut code);

    // Convert byte at REPL_SCALE to BCD at REPL_SCALE_BCD
    let byte_to_scale_bcd = code.len() as u16;
    emit_byte_to_scale_bcd(&mut code);

    // Convert BCD at REPL_SCALE_BCD back to byte and store at REPL_SCALE
    let scale_bcd_to_byte = code.len() as u16;
    emit_scale_bcd_to_byte(&mut code);

    // Apply binary operator (A = op, pops 2 vals, pushes result)
    let apply_op = code.len() as u16;
    emit_repl_apply_op(&mut code, val_pop, val_push, alloc_num, bcd_add, bcd_sub, bcd_mul, bcd_div, bcd_mul10, bcd_copy, scale_bcd_to_byte);

    // Evaluate expression from token buffer
    let evaluate = code.len() as u16;
    emit_repl_evaluate(&mut code, val_push, val_pop, op_push, op_pop, op_empty, op_peek, get_prec, apply_op, byte_to_scale_bcd, alloc_num, bcd_copy);

    // Print BCD number (use the working VM version)
    let print_num = code.len() as u16;
    emit_print_bcd_number(&mut code, acia_out);

    // === Initialization ===
    let init_addr = code.len() as u16;
    // Patch the initial jump
    code[init_patch] = (init_addr & 0xFF) as u8;
    code[init_patch + 1] = (init_addr >> 8) as u8;

    emit_repl_init(&mut code);

    // === Main REPL loop ===
    let repl_loop = code.len() as u16;
    emit_repl_main_loop(&mut code, print_str, print_crlf, getline, tokenize, evaluate, val_pop, print_num, repl_loop);

    // === String constants ===
    let banner_str = code.len() as u16;
    for b in b"bc80 REPL v1.0\r\n" {
        code.push(*b);
    }
    code.push(0);

    let prompt_str = code.len() as u16;
    for b in b"> " {
        code.push(*b);
    }
    code.push(0);

    let error_str = code.len() as u16;
    for b in b"Error\r\n" {
        code.push(*b);
    }
    code.push(0);

    // Patch string addresses in init
    patch_repl_strings(&mut code, init_addr, banner_str, prompt_str, error_str, print_str, repl_loop);

    eprintln!("REPL code size: {} bytes", code.len());

    code
}

fn emit_repl_acia_out(code: &mut Vec<u8>) {
    use opcodes::*;
    // Wait for TX ready, then output A
    code.push(PUSH_AF);
    let wait_loop = code.len() as u16;
    code.push(IN_A_N);
    code.push(ACIA_STATUS_PORT);
    code.push(AND_N);
    code.push(ACIA_TX_READY);
    code.push(JR_Z_N);
    let offset = (wait_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);
    code.push(POP_AF);
    code.push(OUT_N_A);
    code.push(ACIA_DATA_PORT);
    code.push(RET);
}

fn emit_repl_acia_in(code: &mut Vec<u8>) {
    use opcodes::*;
    // Wait for RX ready, then read to A
    let wait_loop = code.len() as u16;
    code.push(IN_A_N);
    code.push(ACIA_STATUS_PORT);
    code.push(AND_N);
    code.push(ACIA_RX_READY);
    code.push(JR_Z_N);
    let offset = (wait_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);
    code.push(IN_A_N);
    code.push(ACIA_DATA_PORT);
    code.push(RET);
}

fn emit_repl_print_str(code: &mut Vec<u8>, acia_out: u16) {
    use opcodes::*;
    // HL = string pointer, print until null
    let loop_start = code.len() as u16;
    code.push(LD_A_HL);
    code.push(OR_A);
    code.push(RET_Z);
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    code.push(INC_HL);
    code.push(JR_N);
    let offset = (loop_start as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);
}

fn emit_repl_print_crlf(code: &mut Vec<u8>, acia_out: u16) {
    use opcodes::*;
    code.push(LD_A_N);
    code.push(0x0D);  // CR
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    code.push(LD_A_N);
    code.push(0x0A);  // LF
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    code.push(RET);
}

fn emit_repl_getline(code: &mut Vec<u8>, acia_in: u16, acia_out: u16) {
    use opcodes::*;
    // Read line into REPL_INPUT_BUF, handle backspace
    code.push(LD_HL_NN);
    emit_u16(code, REPL_INPUT_BUF);
    code.push(LD_B_N);
    code.push(0);  // Character count

    let loop_start = code.len() as u16;
    code.push(CALL_NN);
    emit_u16(code, acia_in);

    // Check for CR
    code.push(CP_N);
    code.push(13);
    let done = jr_placeholder(code, JR_Z_N);

    // Check for LF
    code.push(CP_N);
    code.push(10);
    let done2 = jr_placeholder(code, JR_Z_N);

    // Check for backspace
    code.push(CP_N);
    code.push(8);
    let not_bs = jr_placeholder(code, JR_NZ_N);

    // Handle backspace
    code.push(LD_A_B);
    code.push(OR_A);
    let no_del = jr_placeholder(code, JR_Z_N);  // Nothing to delete
    code.push(DEC_B);
    code.push(DEC_HL);
    // Echo: BS, space, BS
    code.push(LD_A_N);
    code.push(8);
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    code.push(LD_A_N);
    code.push(b' ');
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    code.push(LD_A_N);
    code.push(8);
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    patch_jr(code, no_del);
    code.push(JR_N);
    let back_to_loop = (loop_start as i16 - code.len() as i16 - 1) as i8;
    code.push(back_to_loop as u8);

    patch_jr(code, not_bs);
    // Check buffer full
    code.push(LD_C_A);  // Save char
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(250);
    let not_full = jr_placeholder(code, JR_C_N);
    code.push(JR_N);
    let back_to_loop2 = (loop_start as i16 - code.len() as i16 - 1) as i8;
    code.push(back_to_loop2 as u8);

    patch_jr(code, not_full);
    // Store character and echo
    code.push(LD_A_C);
    code.push(LD_HL_A);
    code.push(INC_HL);
    code.push(INC_B);
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    code.push(JR_N);
    let back_to_loop3 = (loop_start as i16 - code.len() as i16 - 1) as i8;
    code.push(back_to_loop3 as u8);

    // Done - null terminate
    patch_jr(code, done);
    patch_jr(code, done2);
    code.push(XOR_A);
    code.push(LD_HL_A);  // Null terminate
    code.push(LD_A_B);
    code.push(LD_NN_A);
    emit_u16(code, REPL_INPUT_LEN);
    code.push(XOR_A);
    code.push(LD_NN_A);
    emit_u16(code, REPL_INPUT_POS);
    code.push(RET);
}

fn emit_repl_alloc_num(code: &mut Vec<u8>) {
    use opcodes::*;
    // Allocate 28 bytes on heap, return pointer in HL
    code.push(LD_HL_NN_IND);
    emit_u16(code, REPL_HEAP_PTR);
    code.push(PUSH_HL);  // Save current pointer (return value)

    // Add 28 to heap pointer
    code.push(LD_DE_NN);
    emit_u16(code, 28);
    code.push(ADD_HL_DE);
    code.push(LD_NN_HL);
    emit_u16(code, REPL_HEAP_PTR);

    code.push(POP_HL);  // Return allocated pointer
    code.push(RET);
}

fn emit_repl_copy_number(code: &mut Vec<u8>) {
    use opcodes::*;
    // Copy 28-byte REPL BCD number from DE to HL
    // Format: [sign:1][len:1][scale:1][25 packed bytes] = 28 bytes

    code.push(PUSH_HL);
    code.push(PUSH_DE);

    // Use LDIR to copy 28 bytes
    code.push(LD_BC_NN);
    emit_u16(code, 28);
    code.push(EX_DE_HL);  // HL = source, DE = dest
    emit_ldir(code);

    code.push(POP_DE);
    code.push(POP_HL);
    code.push(RET);
}

/// Convert byte at REPL_SCALE to BCD number at REPL_SCALE_BCD
/// Value 0-255 becomes up to 3 decimal digits
/// Uses fixed len=50 format with right-aligned digits (same as parsed numbers)
fn emit_byte_to_scale_bcd(code: &mut Vec<u8>) {
    use opcodes::*;
    // Read the byte
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_SCALE);
    // A = scale value (0-255)

    code.push(LD_HL_NN);
    emit_u16(code, REPL_SCALE_BCD);

    // Initialize BCD structure: sign=0, len=50, scale=0
    code.push(PUSH_AF);           // Save value
    code.push(XOR_A);
    code.push(LD_HL_A);           // sign = 0
    code.push(INC_HL);
    code.push(LD_A_N);
    code.push(50);                // len = 50 (fixed format)
    code.push(LD_HL_A);
    code.push(INC_HL);
    code.push(XOR_A);
    code.push(LD_HL_A);           // scale = 0
    code.push(INC_HL);

    // Zero out the packed digit area (25 bytes)
    code.push(LD_B_N);
    code.push(25);
    let zero_loop = code.len() as u16;
    code.push(XOR_A);
    code.push(LD_HL_A);
    code.push(INC_HL);
    code.push(DJNZ_N);
    let back = (zero_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back as u8);

    code.push(POP_AF);            // Restore value

    // Convert byte to decimal digits: A = value (0-255)
    // D = hundreds, E = tens, result in A = ones
    code.push(LD_D_N);
    code.push(0);                 // D = hundreds (initial)
    code.push(LD_E_N);
    code.push(0);                 // E = tens

    // Count hundreds
    let hundreds_loop = code.len() as u16;
    code.push(CP_N);
    code.push(100);
    let no_more_hundreds = jr_placeholder(code, JR_C_N);
    code.push(SUB_N);
    code.push(100);
    code.push(INC_D);
    code.push(JR_N);
    let back_h = (hundreds_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back_h as u8);

    patch_jr(code, no_more_hundreds);

    // Count tens
    let tens_loop = code.len() as u16;
    code.push(CP_N);
    code.push(10);
    let no_more_tens = jr_placeholder(code, JR_C_N);
    code.push(SUB_N);
    code.push(10);
    code.push(INC_E);
    code.push(JR_N);
    let back_t = (tens_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back_t as u8);

    patch_jr(code, no_more_tens);

    // A = ones, D = hundreds, E = tens
    code.push(LD_C_A);            // C = ones

    // Store digits right-aligned at bytes 26-27 (last 2 packed bytes)
    // Byte 26 = (hundreds << 4) | tens (positions 49-48)
    // Byte 27 = ones << 4          (position 50, rightmost)
    // Actually for single digit values (0-9), only byte 27 low nibble is used
    // But we'll pack all 3 for values up to 255

    code.push(LD_HL_NN);
    emit_u16(code, REPL_SCALE_BCD + 3 + 24);  // byte 27 (offset 3 + 24 = 27)

    // Byte 27: ones in LOW nibble (rightmost position)
    code.push(LD_A_C);            // ones
    code.push(LD_HL_A);           // store ones in low nibble

    // Check if we have tens or hundreds
    code.push(LD_A_D);
    code.push(OR_E);
    code.push(RET_Z);             // Only ones, we're done

    // Byte 27: add tens to high nibble
    code.push(LD_A_E);            // tens
    code.push(ADD_A_A);           // * 2
    code.push(ADD_A_A);           // * 4
    code.push(ADD_A_A);           // * 8
    code.push(ADD_A_A);           // * 16 = shift left 4
    code.push(OR_C);              // combine with ones (C still has ones)
    code.push(LD_HL_A);

    // Check if we have hundreds
    code.push(LD_A_D);
    code.push(OR_A);
    code.push(RET_Z);             // No hundreds, we're done

    // Byte 26: hundreds in LOW nibble
    code.push(DEC_HL);            // point to byte 26
    code.push(LD_A_D);            // hundreds
    code.push(LD_HL_A);           // store hundreds in low nibble

    code.push(RET);
}

/// Convert BCD number at REPL_SCALE_BCD back to byte and store at REPL_SCALE
/// Reads from right-aligned format (len=50, digits in last bytes)
fn emit_scale_bcd_to_byte(code: &mut Vec<u8>) {
    use opcodes::*;
    // Read from the last 2 packed bytes (bytes 26-27)
    // which contain the rightmost digits

    code.push(LD_HL_NN);
    emit_u16(code, REPL_SCALE_BCD + 3 + 24);  // byte 27

    // Byte 27: low nibble = ones, high nibble = tens
    code.push(LD_A_HL);
    code.push(LD_B_A);            // B = packed (tens|ones)
    code.push(AND_N);
    code.push(0x0F);              // A = ones
    code.push(LD_C_A);            // C = ones

    code.push(LD_A_B);
    code.push(RRCA);              // Rotate right 4 times
    code.push(RRCA);
    code.push(RRCA);
    code.push(RRCA);
    code.push(AND_N);
    code.push(0x0F);              // A = tens
    code.push(LD_E_A);            // E = tens

    // Byte 26: low nibble = hundreds
    code.push(DEC_HL);
    code.push(LD_A_HL);
    code.push(AND_N);
    code.push(0x0F);              // A = hundreds
    code.push(LD_D_A);            // D = hundreds

    // Calculate value = hundreds*100 + tens*10 + ones
    // Start with ones
    code.push(LD_A_C);
    code.push(LD_L_A);
    code.push(LD_H_N);
    code.push(0);                 // HL = ones

    // Add tens * 10
    code.push(LD_A_E);            // A = tens
    code.push(OR_A);              // Check if tens = 0
    let skip_tens = jr_placeholder(code, JR_Z_N);
    code.push(LD_B_A);            // B = tens count
    let add_tens_loop = code.len() as u16;
    code.push(LD_DE_NN);
    emit_u16(code, 10);
    code.push(ADD_HL_DE);
    code.push(DJNZ_N);
    let back_tens = (add_tens_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back_tens as u8);

    patch_jr(code, skip_tens);

    // Add hundreds * 100
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_SCALE_BCD + 3 + 23);  // byte 26, reload D
    code.push(AND_N);
    code.push(0x0F);
    code.push(OR_A);
    let skip_hundreds = jr_placeholder(code, JR_Z_N);
    code.push(LD_B_A);            // B = hundreds count
    let add_hundreds_loop = code.len() as u16;
    code.push(LD_DE_NN);
    emit_u16(code, 100);
    code.push(ADD_HL_DE);
    code.push(DJNZ_N);
    let back_hundreds = (add_hundreds_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back_hundreds as u8);

    patch_jr(code, skip_hundreds);

    // L = low byte of result (we assume scale <= 255)
    code.push(LD_A_L);
    code.push(LD_NN_A);
    emit_u16(code, REPL_SCALE);

    code.push(RET);
}

fn emit_repl_parse_num(code: &mut Vec<u8>, alloc_num: u16) {
    use opcodes::*;
    // Parse number from input at REPL_INPUT_POS
    // Returns HL = pointer to BCD number in fixed 50-digit packed format
    // Format: [sign][len=50][scale][25 packed bytes]
    // Numbers are right-aligned: single digit goes in low nibble of byte 27

    // Allocate space (28 bytes)
    code.push(CALL_NN);
    emit_u16(code, alloc_num);
    code.push(PUSH_HL);  // Save BCD pointer [stack: bcd]

    // Initialize header: sign=0, len=50, scale=0
    code.push(XOR_A);
    code.push(LD_HL_A);  // sign = 0
    code.push(INC_HL);
    code.push(LD_A_N);
    code.push(50);       // Fixed 50 digits
    code.push(LD_HL_A);  // len = 50
    code.push(INC_HL);
    code.push(XOR_A);
    code.push(LD_HL_A);  // scale = 0
    code.push(INC_HL);

    // Zero out all 25 packed bytes
    code.push(LD_B_N);
    code.push(25);
    let zero_loop = code.len() as u16;
    code.push(LD_HL_A);  // Store 0
    code.push(INC_HL);
    code.push(DJNZ_N);
    let offset = (zero_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(offset as u8);

    // Get input position, HL = input pointer
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_INPUT_POS);
    code.push(LD_E_A);
    code.push(LD_D_N);
    code.push(0);
    code.push(LD_HL_NN);
    emit_u16(code, REPL_INPUT_BUF);
    code.push(ADD_HL_DE);

    // Count digits and find end position
    code.push(LD_B_N);
    code.push(0);  // B = digit count

    let count_loop = code.len() as u16;
    code.push(LD_A_HL);
    code.push(SUB_N);
    code.push(b'0');
    let count_done = jr_placeholder(code, JR_C_N);
    code.push(CP_N);
    code.push(10);
    let count_done2 = jr_placeholder(code, JR_NC_N);
    code.push(INC_B);
    code.push(INC_HL);
    code.push(JR_N);
    let back = (count_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back as u8);

    patch_jr(code, count_done);
    patch_jr(code, count_done2);
    // HL = one past last digit, B = digit count

    // Update input position
    code.push(PUSH_HL);
    code.push(LD_DE_NN);
    emit_u16(code, REPL_INPUT_BUF);
    code.push(OR_A);
    emit_sbc_hl_de(code);
    code.push(LD_A_L);
    code.push(LD_NN_A);
    emit_u16(code, REPL_INPUT_POS);
    code.push(POP_HL);  // HL = one past last digit

    // If no digits, return zero
    code.push(LD_A_B);
    code.push(OR_A);
    let has_digits = jr_placeholder(code, JR_NZ_N);
    code.push(POP_HL);  // Return BCD pointer
    code.push(RET);

    patch_jr(code, has_digits);

    // Get BCD pointer, calculate position for last packed byte (offset 27)
    code.push(POP_DE);   // DE = BCD pointer [stack: empty]
    code.push(PUSH_DE);  // Save for return [stack: bcd]
    code.push(LD_A_N);
    code.push(27);
    code.push(ADD_A_E);
    code.push(LD_E_A);
    let no_carry = jr_placeholder(code, JR_NC_N);
    code.push(INC_D);
    patch_jr(code, no_carry);
    // DE = pointer to last packed byte (byte 27 = digits 49-50)

    // HL = one past last digit, B = count, go back to last digit
    code.push(DEC_HL);

    // Save original count's parity to temp location
    // Position = (original_count - B), if even -> low nibble, if odd -> high nibble
    // (original_count XOR B) has same parity as (original_count - B)
    code.push(LD_A_B);
    code.push(AND_N);
    code.push(1);
    code.push(LD_NN_A);
    emit_u16(code, REPL_TEMP);  // Save parity of original count

    // Pack digits from right to left
    let pack_loop = code.len() as u16;
    code.push(LD_A_HL);
    code.push(SUB_N);
    code.push(b'0');
    code.push(LD_C_A);   // C = digit (0-9)

    // Check position parity: (original_parity XOR B) & 1
    // If 0 -> low nibble (even position from right)
    // If 1 -> high nibble (odd position from right)
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_TEMP);
    code.push(XOR_B);
    code.push(AND_N);
    code.push(1);
    let is_high_nibble = jr_placeholder(code, JR_NZ_N);

    // Even count remaining: store in LOW nibble (rightmost digit position)
    code.push(LD_A_DE);
    code.push(AND_N);
    code.push(0xF0);     // Keep high nibble
    code.push(OR_C);     // Add low nibble
    code.push(LD_DE_A);
    let done_digit = jr_placeholder(code, JR_N);

    patch_jr(code, is_high_nibble);
    // Odd count remaining: store in HIGH nibble
    code.push(LD_A_C);
    code.push(RLA);
    code.push(RLA);
    code.push(RLA);
    code.push(RLA);
    code.push(LD_C_A);
    code.push(LD_A_DE);
    code.push(AND_N);
    code.push(0x0F);     // Keep low nibble
    code.push(OR_C);     // Add high nibble
    code.push(LD_DE_A);
    code.push(DEC_DE);   // Move to previous packed byte

    patch_jr(code, done_digit);
    code.push(DEC_B);
    let pack_done = jr_placeholder(code, JR_Z_N);
    code.push(DEC_HL);
    code.push(JR_N);
    let back2 = (pack_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back2 as u8);

    patch_jr(code, pack_done);
    code.push(POP_HL);   // Return BCD pointer
    code.push(RET);
}

fn emit_repl_tokenize(code: &mut Vec<u8>, parse_num: u16) {
    use opcodes::*;
    // Tokenize REPL_INPUT_BUF into REPL_TOKEN_BUF

    // Reset token count
    code.push(XOR_A);
    code.push(LD_NN_A);
    emit_u16(code, REPL_TOKEN_CNT);
    code.push(LD_NN_A);
    emit_u16(code, REPL_INPUT_POS);

    code.push(LD_HL_NN);
    emit_u16(code, REPL_INPUT_BUF);
    code.push(LD_DE_NN);
    emit_u16(code, REPL_TOKEN_BUF);

    let tok_loop = code.len() as u16;
    code.push(LD_A_HL);
    code.push(OR_A);
    let tok_done = jp_z_placeholder(code);  // Use JP Z for long jump

    // Skip whitespace
    code.push(CP_N);
    code.push(b' ');
    let not_space = jr_placeholder(code, JR_NZ_N);
    code.push(INC_HL);
    // Update input pos
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_INPUT_POS);
    code.push(INC_A);
    code.push(LD_NN_A);
    emit_u16(code, REPL_INPUT_POS);
    code.push(JR_N);
    let back = (tok_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back as u8);

    patch_jr(code, not_space);

    // Check for digit
    code.push(LD_A_HL);
    code.push(SUB_N);
    code.push(b'0');
    let not_digit = jr_placeholder(code, JR_C_N);
    code.push(CP_N);
    code.push(10);
    let is_digit = jr_placeholder(code, JR_C_N);

    patch_jr(code, not_digit);
    // Check for decimal point starting a number
    code.push(LD_A_HL);
    code.push(CP_N);
    code.push(b'.');
    let not_num = jr_placeholder(code, JR_NZ_N);

    patch_jr(code, is_digit);
    // Parse number
    code.push(PUSH_HL);
    code.push(PUSH_DE);
    // Calculate input pos from HL
    code.push(LD_DE_NN);
    emit_u16(code, REPL_INPUT_BUF);
    code.push(OR_A);
    emit_sbc_hl_de(code);
    code.push(LD_A_L);
    code.push(LD_NN_A);
    emit_u16(code, REPL_INPUT_POS);
    code.push(CALL_NN);
    emit_u16(code, parse_num);  // Returns HL = BCD pointer
    code.push(LD_B_H);
    code.push(LD_C_L);  // BC = BCD pointer
    code.push(POP_DE);
    // Store token
    code.push(LD_A_N);
    code.push(TOK_NUMBER);
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(LD_A_C);
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(LD_A_B);
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(XOR_A);
    code.push(LD_DE_A);
    code.push(INC_DE);
    // Increment token count
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_TOKEN_CNT);
    code.push(INC_A);
    code.push(LD_NN_A);
    emit_u16(code, REPL_TOKEN_CNT);
    // Update HL from input pos
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_INPUT_POS);
    code.push(LD_L_A);
    code.push(LD_H_N);
    code.push(0);
    code.push(LD_BC_NN);
    emit_u16(code, REPL_INPUT_BUF);
    code.push(ADD_HL_BC);
    code.push(POP_AF);  // Discard old HL
    code.push(JR_N);
    let back2 = (tok_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back2 as u8);

    patch_jr(code, not_num);
    // Check for operators
    // NOTE: Use JP Z instead of JR Z because distance to store_op can exceed 127 bytes
    code.push(LD_A_HL);
    code.push(LD_B_N);
    code.push(TOK_PLUS);
    code.push(CP_N);
    code.push(b'+');
    let store_op = jp_z_placeholder(code);
    code.push(LD_B_N);
    code.push(TOK_MINUS);
    code.push(CP_N);
    code.push(b'-');
    let store_op2 = jp_z_placeholder(code);
    code.push(LD_B_N);
    code.push(TOK_STAR);
    code.push(CP_N);
    code.push(b'*');
    let store_op3 = jp_z_placeholder(code);
    code.push(LD_B_N);
    code.push(TOK_SLASH);
    code.push(CP_N);
    code.push(b'/');
    let store_op4 = jp_z_placeholder(code);
    code.push(LD_B_N);
    code.push(TOK_LPAREN);
    code.push(CP_N);
    code.push(b'(');
    let store_op5 = jp_z_placeholder(code);
    code.push(LD_B_N);
    code.push(TOK_RPAREN);
    code.push(CP_N);
    code.push(b')');
    let store_op6 = jp_z_placeholder(code);
    // Check for '=' (assignment)
    code.push(LD_B_N);
    code.push(TOK_ASSIGN);
    code.push(CP_N);
    code.push(b'=');
    let store_op7 = jp_z_placeholder(code);

    // Check for variable (a-z)
    code.push(LD_A_HL);
    code.push(SUB_N);
    code.push(b'a');
    let not_var = jr_placeholder(code, JR_C_N);  // char < 'a'
    code.push(CP_N);
    code.push(26);  // Check if < 26 (i.e., <= 'z')
    let is_var = jr_placeholder(code, JR_C_N);

    patch_jr(code, not_var);
    // Unknown character - skip it
    code.push(INC_HL);
    // Use JP instead of JR - too far for relative jump
    code.push(JP_NN);
    emit_u16(code, tok_loop);

    // Store variable token
    patch_jr(code, is_var);
    // A = (char - 'a') = variable index (0-25)
    // But first check if this is "scale" keyword
    code.push(CP_N);
    code.push(b's' - b'a');      // Is it 's'?
    let not_scale = jr_placeholder(code, JR_NZ_N);

    // Check for "scale" - compare next 4 chars with "cale"
    code.push(PUSH_HL);          // Save current position
    code.push(INC_HL);
    code.push(LD_A_HL);
    code.push(CP_N);
    code.push(b'c');
    let not_scale2 = jr_placeholder(code, JR_NZ_N);
    code.push(INC_HL);
    code.push(LD_A_HL);
    code.push(CP_N);
    code.push(b'a');
    let not_scale3 = jr_placeholder(code, JR_NZ_N);
    code.push(INC_HL);
    code.push(LD_A_HL);
    code.push(CP_N);
    code.push(b'l');
    let not_scale4 = jr_placeholder(code, JR_NZ_N);
    code.push(INC_HL);
    code.push(LD_A_HL);
    code.push(CP_N);
    code.push(b'e');
    let not_scale5 = jr_placeholder(code, JR_NZ_N);

    // It's "scale"! Store as TOK_VARIABLE with index 26
    code.push(POP_AF);           // Discard saved HL
    // HL is at 'e', will be incremented at the end like regular variables
    code.push(LD_A_N);
    code.push(TOK_VARIABLE);     // Treat scale like a variable
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(LD_A_N);
    code.push(26);               // Scale uses variable slot 26
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(XOR_A);
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(LD_DE_A);
    code.push(INC_DE);
    // Increment token count
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_TOKEN_CNT);
    code.push(INC_A);
    code.push(LD_NN_A);
    emit_u16(code, REPL_TOKEN_CNT);
    code.push(INC_HL);           // Move past last char
    code.push(JP_NN);
    emit_u16(code, tok_loop);

    // Not "scale", restore and treat as variable 's'
    patch_jr(code, not_scale2);
    patch_jr(code, not_scale3);
    patch_jr(code, not_scale4);
    patch_jr(code, not_scale5);
    code.push(POP_HL);           // Restore position

    patch_jr(code, not_scale);
    // A is already variable index from earlier (char - 'a')
    // But we clobbered it checking for 'scale', reload
    code.push(LD_A_HL);
    code.push(SUB_N);
    code.push(b'a');

    code.push(LD_C_A);  // C = variable index
    code.push(LD_A_N);
    code.push(TOK_VARIABLE);
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(LD_A_C);  // A = index
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(XOR_A);
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(LD_DE_A);
    code.push(INC_DE);
    // Increment token count
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_TOKEN_CNT);
    code.push(INC_A);
    code.push(LD_NN_A);
    emit_u16(code, REPL_TOKEN_CNT);
    code.push(INC_HL);
    code.push(JP_NN);
    emit_u16(code, tok_loop);

    // Store single-char operator
    patch_jp(code, store_op);
    patch_jp(code, store_op2);
    patch_jp(code, store_op3);
    patch_jp(code, store_op4);
    patch_jp(code, store_op5);
    patch_jp(code, store_op6);
    patch_jp(code, store_op7);
    code.push(LD_A_B);
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(XOR_A);
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(LD_DE_A);
    code.push(INC_DE);
    code.push(LD_DE_A);
    code.push(INC_DE);
    // Increment token count
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_TOKEN_CNT);
    code.push(INC_A);
    code.push(LD_NN_A);
    emit_u16(code, REPL_TOKEN_CNT);
    code.push(INC_HL);
    // Use JP instead of JR - too far for relative jump
    code.push(JP_NN);
    emit_u16(code, tok_loop);

    // Done
    patch_jp(code, tok_done);  // Patch the long JP Z jump
    // Store EOF token
    code.push(LD_A_N);
    code.push(TOK_EOF);
    code.push(LD_DE_A);
    code.push(RET);
}

fn emit_repl_val_push(code: &mut Vec<u8>) {
    use opcodes::*;
    // Push HL onto value stack
    code.push(PUSH_HL);
    code.push(LD_HL_NN_IND);
    emit_u16(code, REPL_VAL_SP);
    code.push(POP_DE);
    code.push(LD_HL_E);
    code.push(INC_HL);
    code.push(LD_HL_D);
    code.push(INC_HL);
    code.push(LD_NN_HL);
    emit_u16(code, REPL_VAL_SP);
    code.push(RET);
}

fn emit_repl_val_pop(code: &mut Vec<u8>) {
    use opcodes::*;
    // Pop value from stack, return in HL
    code.push(LD_HL_NN_IND);
    emit_u16(code, REPL_VAL_SP);
    code.push(DEC_HL);
    code.push(LD_D_HL);
    code.push(DEC_HL);
    code.push(LD_E_HL);
    code.push(LD_NN_HL);
    emit_u16(code, REPL_VAL_SP);
    code.push(EX_DE_HL);
    code.push(RET);
}

fn emit_repl_op_push(code: &mut Vec<u8>) {
    use opcodes::*;
    // Push A onto operator stack
    code.push(LD_HL_NN_IND);
    emit_u16(code, REPL_OP_SP);
    code.push(LD_HL_A);
    code.push(INC_HL);
    code.push(LD_NN_HL);
    emit_u16(code, REPL_OP_SP);
    code.push(RET);
}

fn emit_repl_op_pop(code: &mut Vec<u8>) {
    use opcodes::*;
    // Pop from operator stack, return in A
    code.push(LD_HL_NN_IND);
    emit_u16(code, REPL_OP_SP);
    code.push(DEC_HL);
    code.push(LD_A_HL);
    code.push(LD_NN_HL);
    emit_u16(code, REPL_OP_SP);
    code.push(RET);
}

fn emit_repl_op_empty(code: &mut Vec<u8>) {
    use opcodes::*;
    // Check if operator stack is empty (Z set if empty)
    code.push(LD_HL_NN_IND);
    emit_u16(code, REPL_OP_SP);
    code.push(LD_DE_NN);
    emit_u16(code, REPL_OP_STACK);
    code.push(OR_A);
    emit_sbc_hl_de(code);
    code.push(LD_A_L);
    code.push(OR_H);
    code.push(RET);
}

fn emit_repl_op_peek(code: &mut Vec<u8>) {
    use opcodes::*;
    // Peek top of operator stack, return in A
    code.push(LD_HL_NN_IND);
    emit_u16(code, REPL_OP_SP);
    code.push(DEC_HL);
    code.push(LD_A_HL);
    code.push(RET);
}

fn emit_repl_get_prec(code: &mut Vec<u8>) {
    use opcodes::*;
    // Get precedence for operator in A, return in A
    // +/- = 1, */ = 2, ( = 0
    code.push(CP_N);
    code.push(TOK_PLUS);
    let not_plus = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_N);
    code.push(1);
    code.push(RET);

    patch_jr(code, not_plus);
    code.push(CP_N);
    code.push(TOK_MINUS);
    let not_minus = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_N);
    code.push(1);
    code.push(RET);

    patch_jr(code, not_minus);
    code.push(CP_N);
    code.push(TOK_STAR);
    let not_star = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_N);
    code.push(2);
    code.push(RET);

    patch_jr(code, not_star);
    code.push(CP_N);
    code.push(TOK_SLASH);
    let not_slash = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_N);
    code.push(2);
    code.push(RET);

    patch_jr(code, not_slash);
    // Default (including LPAREN) = 0
    code.push(XOR_A);
    code.push(RET);
}

fn emit_repl_apply_op(code: &mut Vec<u8>, val_pop: u16, val_push: u16, alloc_num: u16,
                      bcd_add: u16, bcd_sub: u16, bcd_mul: u16, bcd_div: u16, bcd_mul10: u16, bcd_copy: u16,
                      _scale_bcd_to_byte: u16) {
    use opcodes::*;
    // Apply operator in A to top two values on stack
    // Strategy: copy left to result, then apply operation with right
    // BCD add: (HL) = (DE) + (HL), so result = right + left = left + right
    // BCD sub: (HL) = (HL) - (DE), so result = left - right
    // Assignment: copy right to left, push left

    // Check for assignment first (needs different handling)
    code.push(CP_N);
    code.push(TOK_ASSIGN);
    let not_assign = jr_placeholder(code, JR_NZ_N);

    // === ASSIGNMENT HANDLING ===
    // Pop right operand (the value)
    code.push(CALL_NN);
    emit_u16(code, val_pop);
    code.push(PUSH_HL);  // [stack: right]

    // Pop left operand (the variable address)
    code.push(CALL_NN);
    emit_u16(code, val_pop);
    // HL = left (dest), [stack: right]

    code.push(POP_DE);   // DE = right (source), [stack: empty]
    code.push(PUSH_HL);  // Save left (result) [stack: left]

    // Copy right to left: HL=dest (left), DE=source (right)
    code.push(EX_DE_HL); // Now HL=source, DE=dest for bcd_copy (HL=dest, DE=src)
    code.push(EX_DE_HL); // Swap back - bcd_copy needs HL=dest, DE=src
    // Actually bcd_copy does: copy from DE to HL
    // So HL = left (dest), DE = right (source) is correct
    code.push(CALL_NN);
    emit_u16(code, bcd_copy);

    // After bcd_copy, HL is corrupted (points past data due to LDIR).
    // left was saved on stack before the copy.
    // Check if left == scale (slot 26). If so, sync BCD to REPL_SCALE byte.
    // REPL_SCALE_BCD = REPL_VARS + 26*28 = 0x8400 + 0x2E8 = 0x86E8
    code.push(POP_HL);           // HL = left [stack: empty]
    code.push(PUSH_HL);          // Re-save [stack: left]
    code.push(LD_DE_NN);
    emit_u16(code, REPL_VARS + 26 * 28);  // Scale BCD address
    code.push(LD_A_L);
    code.push(XOR_E);
    let not_scale = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_H);
    code.push(XOR_D);
    let not_scale2 = jr_placeholder(code, JR_NZ_N);

    // It's scale! Extract byte value from last packed byte
    // HL = scale BCD, [stack: left]
    code.push(LD_BC_NN);
    emit_u16(code, 27);          // Point to last byte (offset 27)
    code.push(ADD_HL_BC);
    code.push(LD_A_HL);          // A = last packed byte (2 BCD digits, 0-99)
    // Convert packed BCD to binary: high_digit * 10 + low_digit
    code.push(LD_B_A);           // Save packed
    code.push(AND_N);
    code.push(0x0F);             // A = low digit
    code.push(LD_C_A);           // C = low digit
    code.push(LD_A_B);           // A = packed
    code.push(RRCA);
    code.push(RRCA);
    code.push(RRCA);
    code.push(RRCA);
    code.push(AND_N);
    code.push(0x0F);             // A = high digit
    // A * 10 = A * 8 + A * 2
    code.push(LD_B_A);           // B = high digit
    code.push(ADD_A_A);          // A = 2 * high
    code.push(ADD_A_A);          // A = 4 * high
    code.push(ADD_A_B);          // A = 5 * high
    code.push(ADD_A_A);          // A = 10 * high
    code.push(ADD_A_C);          // A = 10 * high + low
    code.push(LD_NN_A);
    emit_u16(code, REPL_SCALE);  // Store to single-byte REPL_SCALE

    patch_jr(code, not_scale);
    patch_jr(code, not_scale2);
    // Either path: stack has [left]

    // Push result (left, which now contains right's value)
    code.push(POP_HL);   // HL = left [stack: empty]
    code.push(CALL_NN);
    emit_u16(code, val_push);
    code.push(RET);

    // === NORMAL OPERATOR HANDLING ===
    patch_jr(code, not_assign);

    code.push(PUSH_AF);  // Save operator [stack: op]

    // Pop right operand
    code.push(CALL_NN);
    emit_u16(code, val_pop);
    code.push(PUSH_HL);  // [stack: right, op]

    // Pop left operand
    code.push(CALL_NN);
    emit_u16(code, val_pop);
    // HL = left, [stack: right, op]

    // Allocate result
    code.push(PUSH_HL);  // Save left [stack: left, right, op]
    code.push(CALL_NN);
    emit_u16(code, alloc_num);
    // HL = result ptr, [stack: left, right, op]

    code.push(POP_DE);   // DE = left, [stack: right, op]
    code.push(PUSH_HL);  // Save result [stack: result, right, op]

    // Copy left to result: HL=dest (result), DE=source (left)
    code.push(CALL_NN);
    emit_u16(code, bcd_copy);

    // Set up for BCD operation: HL = result (has left's data), DE = right
    code.push(POP_HL);   // HL = result, [stack: right, op]
    code.push(POP_DE);   // DE = right, [stack: op]
    code.push(POP_AF);   // A = op [stack: empty]
    code.push(PUSH_HL);  // Save result [stack: result]
    // Now: HL = result (has left), DE = right, A = operator

    // Dispatch based on operator
    code.push(CP_N);
    code.push(TOK_PLUS);
    let do_add = jr_placeholder(code, JR_Z_N);
    code.push(CP_N);
    code.push(TOK_MINUS);
    let do_sub = jr_placeholder(code, JR_Z_N);
    code.push(CP_N);
    code.push(TOK_STAR);
    let do_mul = jr_placeholder(code, JR_Z_N);
    code.push(CP_N);
    code.push(TOK_SLASH);
    let do_div = jr_placeholder(code, JR_Z_N);

    // Unknown op - result already has left's value
    let done = jr_placeholder(code, JR_N);

    // Add: result = left + right
    // bcd_add: (HL) = (DE) + (HL), so result = right + result = right + left
    patch_jr(code, do_add);
    code.push(CALL_NN);
    emit_u16(code, bcd_add);
    let done2 = jr_placeholder(code, JR_N);

    // Sub: result = left - right
    // bcd_sub: (HL) = (HL) - (DE), so result = result - right = left - right
    patch_jr(code, do_sub);
    code.push(CALL_NN);
    emit_u16(code, bcd_sub);
    let done3 = jr_placeholder(code, JR_N);

    // Mul: result = left * right
    patch_jr(code, do_mul);
    code.push(CALL_NN);
    emit_u16(code, bcd_mul);
    let done4 = jr_placeholder(code, JR_N);

    // Div: result = left / right (with scale-aware precision)
    patch_jr(code, do_div);
    // Before dividing, multiply dividend by 10^scale for decimal precision
    // HL = dividend (result), DE = divisor
    // Save DE (divisor)
    code.push(PUSH_DE);
    // Read REPL_SCALE
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_SCALE);
    code.push(OR_A);             // Check if scale = 0
    let skip_mul10 = jr_placeholder(code, JR_Z_N);
    code.push(LD_B_A);           // B = scale (loop counter)
    let mul10_loop = code.len() as u16;
    code.push(PUSH_BC);          // Save counter
    code.push(CALL_NN);
    emit_u16(code, bcd_mul10);   // Multiply dividend by 10
    code.push(POP_BC);
    code.push(DJNZ_N);
    let back = (mul10_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back as u8);
    patch_jr(code, skip_mul10);
    // Restore DE (divisor)
    code.push(POP_DE);
    // Now do the integer division
    code.push(CALL_NN);
    emit_u16(code, bcd_div);
    // After division, set result scale byte to REPL_SCALE
    // HL = result (bcd_div returns result in HL)
    code.push(PUSH_HL);          // Save result
    code.push(INC_HL);
    code.push(INC_HL);           // HL = result + 2 (scale byte)
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_SCALE);
    code.push(LD_HL_A);          // Store scale in result
    code.push(POP_HL);           // Restore result pointer

    patch_jr(code, done);
    patch_jr(code, done2);
    patch_jr(code, done3);
    patch_jr(code, done4);

    // Get result pointer and push
    code.push(POP_HL);   // HL = result [stack: empty]
    code.push(CALL_NN);
    emit_u16(code, val_push);
    code.push(RET);
}

fn emit_repl_evaluate(code: &mut Vec<u8>, val_push: u16, val_pop: u16, op_push: u16, op_pop: u16, op_empty: u16, op_peek: u16, get_prec: u16, apply_op: u16, byte_to_scale_bcd: u16, alloc_num: u16, bcd_copy: u16) {
    use opcodes::*;
    // Shunting-yard expression evaluator
    // Reads from REPL_TOKEN_BUF

    // Reset stacks
    code.push(LD_HL_NN);
    emit_u16(code, REPL_VAL_STACK);
    code.push(LD_NN_HL);
    emit_u16(code, REPL_VAL_SP);
    code.push(LD_HL_NN);
    emit_u16(code, REPL_OP_STACK);
    code.push(LD_NN_HL);
    emit_u16(code, REPL_OP_SP);

    // IX = token pointer
    code.push(LD_HL_NN);
    emit_u16(code, REPL_TOKEN_BUF);
    code.push(PUSH_HL);
    emit_pop_ix(code);

    let eval_loop = code.len() as u16;
    // Get token type
    emit_ld_a_ix_d(code, 0);

    // Check EOF - use JP Z for long jump
    code.push(OR_A);
    let flush_ops = jp_z_placeholder(code);

    // Check NUMBER
    code.push(CP_N);
    code.push(TOK_NUMBER);
    let not_num = jr_placeholder(code, JR_NZ_N);
    // Get BCD pointer from token bytes 1-2
    emit_ld_l_ix_d(code, 1);
    emit_ld_h_ix_d(code, 2);
    code.push(CALL_NN);
    emit_u16(code, val_push);
    // Advance token pointer by 4
    code.push(LD_BC_NN);
    emit_u16(code, 4);
    emit_add_ix_bc(code);
    code.push(JR_N);
    let back = (eval_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back as u8);

    patch_jr(code, not_num);
    // Check VARIABLE
    code.push(CP_N);
    code.push(TOK_VARIABLE);
    let not_var = jr_placeholder(code, JR_NZ_N);
    // Get variable index from token byte 1
    emit_ld_a_ix_d(code, 1);
    // Calculate variable address: REPL_VARS + index * 28
    // A = index (0-25)
    code.push(LD_L_A);
    code.push(LD_H_N);
    code.push(0);            // HL = index
    code.push(ADD_HL_HL);    // HL = 2*index
    code.push(ADD_HL_HL);    // HL = 4*index
    code.push(LD_D_H);
    code.push(LD_E_L);       // DE = 4*index
    code.push(ADD_HL_HL);    // HL = 8*index
    code.push(ADD_HL_HL);    // HL = 16*index
    code.push(ADD_HL_HL);    // HL = 32*index
    code.push(OR_A);         // Clear carry
    emit_sbc_hl_de(code);    // HL = 28*index
    code.push(LD_DE_NN);
    emit_u16(code, REPL_VARS);
    code.push(ADD_HL_DE);    // HL = REPL_VARS + 28*index
    code.push(CALL_NN);
    emit_u16(code, val_push);
    // Advance token pointer by 4
    code.push(LD_BC_NN);
    emit_u16(code, 4);
    emit_add_ix_bc(code);
    // Use JP instead of JR to avoid offset overflow
    code.push(JP_NN);
    emit_u16(code, eval_loop);

    patch_jr(code, not_var);
    // Check SCALE - treat it like variable index 26
    code.push(CP_N);
    code.push(TOK_SCALE);
    let not_scale = jr_placeholder(code, JR_NZ_N);
    // Use same address calculation as VARIABLE with index 26
    code.push(LD_A_N);
    code.push(26);  // Scale is "variable" 26
    // Calculate variable address: REPL_VARS + index * 28
    code.push(LD_L_A);
    code.push(LD_H_N);
    code.push(0);            // HL = index
    code.push(ADD_HL_HL);    // HL = 2*index
    code.push(ADD_HL_HL);    // HL = 4*index
    code.push(LD_D_H);
    code.push(LD_E_L);       // DE = 4*index
    code.push(ADD_HL_HL);    // HL = 8*index
    code.push(ADD_HL_HL);    // HL = 16*index
    code.push(ADD_HL_HL);    // HL = 32*index
    code.push(OR_A);         // Clear carry
    emit_sbc_hl_de(code);    // HL = 28*index
    code.push(LD_DE_NN);
    emit_u16(code, REPL_VARS);
    code.push(ADD_HL_DE);    // HL = REPL_VARS + 28*index
    code.push(CALL_NN);
    emit_u16(code, val_push);
    // Advance token pointer by 4
    code.push(LD_BC_NN);
    emit_u16(code, 4);
    emit_add_ix_bc(code);
    code.push(JP_NN);
    emit_u16(code, eval_loop);

    patch_jr(code, not_scale);
    // Check LPAREN
    code.push(CP_N);
    code.push(TOK_LPAREN);
    let not_lparen = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_N);
    code.push(TOK_LPAREN);
    code.push(CALL_NN);
    emit_u16(code, op_push);
    code.push(LD_BC_NN);
    emit_u16(code, 4);
    emit_add_ix_bc(code);
    code.push(JR_N);
    let back2 = (eval_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back2 as u8);

    patch_jr(code, not_lparen);
    // Check RPAREN
    code.push(CP_N);
    code.push(TOK_RPAREN);
    let not_rparen = jr_placeholder(code, JR_NZ_N);
    // Pop and apply until LPAREN
    let rparen_loop = code.len() as u16;
    code.push(CALL_NN);
    emit_u16(code, op_peek);
    code.push(CP_N);
    code.push(TOK_LPAREN);
    let rparen_done = jr_placeholder(code, JR_Z_N);
    code.push(CALL_NN);
    emit_u16(code, op_pop);
    code.push(CALL_NN);
    emit_u16(code, apply_op);
    code.push(JR_N);
    let back3 = (rparen_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back3 as u8);
    patch_jr(code, rparen_done);
    code.push(CALL_NN);
    emit_u16(code, op_pop);  // Discard LPAREN
    code.push(LD_BC_NN);
    emit_u16(code, 4);
    emit_add_ix_bc(code);
    code.push(JR_N);
    let back4 = (eval_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back4 as u8);

    patch_jr(code, not_rparen);
    // It's an operator - handle precedence
    code.push(LD_C_A);  // C = current operator
    code.push(CALL_NN);
    emit_u16(code, get_prec);
    code.push(LD_B_A);  // B = current precedence

    let prec_loop = code.len() as u16;
    code.push(CALL_NN);
    emit_u16(code, op_empty);
    let push_op = jr_placeholder(code, JR_Z_N);
    code.push(CALL_NN);
    emit_u16(code, op_peek);
    code.push(CP_N);
    code.push(TOK_LPAREN);
    let push_op2 = jr_placeholder(code, JR_Z_N);
    code.push(CALL_NN);
    emit_u16(code, get_prec);
    code.push(CP_B);
    let push_op3 = jr_placeholder(code, JR_C_N);  // Stack has lower prec
    // Pop and apply
    code.push(CALL_NN);
    emit_u16(code, op_pop);
    code.push(CALL_NN);
    emit_u16(code, apply_op);
    code.push(JR_N);
    let back5 = (prec_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back5 as u8);

    patch_jr(code, push_op);
    patch_jr(code, push_op2);
    patch_jr(code, push_op3);
    code.push(LD_A_C);
    code.push(CALL_NN);
    emit_u16(code, op_push);
    code.push(LD_BC_NN);
    emit_u16(code, 4);
    emit_add_ix_bc(code);
    // Use JP instead of JR - too far for relative jump
    code.push(JP_NN);
    emit_u16(code, eval_loop);

    // Flush remaining operators
    patch_jp(code, flush_ops);
    let flush_loop = code.len() as u16;
    code.push(CALL_NN);
    emit_u16(code, op_empty);
    code.push(RET_Z);
    code.push(CALL_NN);
    emit_u16(code, op_pop);
    code.push(CP_N);
    code.push(TOK_LPAREN);
    let skip_lparen = jr_placeholder(code, JR_Z_N);
    code.push(CALL_NN);
    emit_u16(code, apply_op);
    patch_jr(code, skip_lparen);
    code.push(JR_N);
    let back7 = (flush_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back7 as u8);
}

fn emit_repl_print_num(code: &mut Vec<u8>, acia_out: u16) {
    use opcodes::*;
    // Print BCD number at HL
    // Format: [sign][len][scale][digits...]

    code.push(PUSH_HL);

    // Check sign
    code.push(LD_A_HL);
    code.push(AND_N);
    code.push(0x80);
    let not_neg = jr_placeholder(code, JR_Z_N);
    code.push(LD_A_N);
    code.push(b'-');
    code.push(CALL_NN);
    emit_u16(code, acia_out);

    patch_jr(code, not_neg);
    code.push(POP_HL);
    code.push(INC_HL);
    code.push(LD_B_HL);  // B = digit count
    code.push(INC_HL);
    code.push(LD_C_HL);  // C = scale (unused for now)
    code.push(INC_HL);

    // Skip leading zeros
    code.push(LD_E_N);
    code.push(0);  // E = printed flag

    let print_loop = code.len() as u16;
    code.push(LD_A_B);
    code.push(OR_A);
    code.push(RET_Z);

    code.push(LD_A_HL);
    // Skip leading zero if E=0 and B>1
    code.push(OR_A);
    let not_zero = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_E);
    code.push(OR_A);
    let already_printed = jr_placeholder(code, JR_NZ_N);
    code.push(LD_A_B);
    code.push(CP_N);
    code.push(1);
    let is_last = jr_placeholder(code, JR_Z_N);
    // Skip this zero
    code.push(INC_HL);
    code.push(DEC_B);
    code.push(JR_N);
    let back = (print_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back as u8);

    patch_jr(code, not_zero);
    patch_jr(code, already_printed);
    patch_jr(code, is_last);
    // Print digit
    code.push(LD_A_HL);
    code.push(ADD_A_N);
    code.push(b'0');
    code.push(CALL_NN);
    emit_u16(code, acia_out);
    code.push(LD_E_N);
    code.push(1);  // Mark as printed
    code.push(INC_HL);
    code.push(DEC_B);
    code.push(JR_N);
    let back2 = (print_loop as i16 - code.len() as i16 - 1) as i8;
    code.push(back2 as u8);
}

fn emit_repl_init(code: &mut Vec<u8>) {
    use opcodes::*;

    // Disable interrupts, set stack
    code.push(DI);
    code.push(LD_SP_NN);
    emit_u16(code, STACK_TOP);

    // Initialize heap pointer
    code.push(LD_HL_NN);
    emit_u16(code, REPL_HEAP);
    code.push(LD_NN_HL);
    emit_u16(code, REPL_HEAP_PTR);

    // Initialize scale = 0
    code.push(XOR_A);
    code.push(LD_NN_A);
    emit_u16(code, REPL_SCALE);

    // NOTE: Scale (slot 26) is NOT pre-initialized like other variables

    // Print banner (address will be patched)
    code.push(LD_HL_NN);
    emit_u16(code, 0);  // Placeholder for banner address
    code.push(CALL_NN);
    emit_u16(code, 0);  // Placeholder for print_str
}

fn emit_repl_main_loop(code: &mut Vec<u8>, print_str: u16, print_crlf: u16, getline: u16, tokenize: u16, evaluate: u16, val_pop: u16, print_num: u16, repl_loop: u16) {
    use opcodes::*;

    // Print prompt
    code.push(LD_HL_NN);
    emit_u16(code, 0);  // Placeholder for prompt address
    code.push(CALL_NN);
    emit_u16(code, print_str);

    // Get line
    code.push(CALL_NN);
    emit_u16(code, getline);

    // Check if empty
    code.push(LD_A_NN_IND);
    emit_u16(code, REPL_INPUT_LEN);
    code.push(OR_A);
    code.push(JP_Z_NN);
    emit_u16(code, repl_loop);

    // Tokenize
    code.push(CALL_NN);
    emit_u16(code, tokenize);

    // Evaluate
    code.push(CALL_NN);
    emit_u16(code, evaluate);

    // Pop result
    code.push(CALL_NN);
    emit_u16(code, val_pop);

    // Print result
    code.push(CALL_NN);
    emit_u16(code, print_num);

    // Print newline
    code.push(CALL_NN);
    emit_u16(code, print_crlf);

    // Loop
    code.push(JP_NN);
    emit_u16(code, repl_loop);
}

fn patch_repl_strings(code: &mut Vec<u8>, init_addr: u16, banner_str: u16, prompt_str: u16, _error_str: u16, print_str: u16, repl_loop: u16) {
    // Find and patch string addresses in init code
    // The init code has:
    //   LD HL, banner_addr
    //   CALL print_str
    // and the main loop has:
    //   LD HL, prompt_addr
    //   CALL print_str

    // Init code structure:
    // DI; LD SP,nn; LD HL,heap; LD (heap_ptr),HL; XOR A; LD (scale),A
    // That's: 1 + 3 + 3 + 3 + 1 + 3 = 14 bytes
    // Then: LD HL,nn (banner) = 3 bytes, CALL nn (print_str) = 3 bytes

    let banner_patch = init_addr as usize + 14 + 1;  // +1 for LD HL opcode
    code[banner_patch] = (banner_str & 0xFF) as u8;
    code[banner_patch + 1] = (banner_str >> 8) as u8;

    let print_str_patch = init_addr as usize + 14 + 3 + 1;  // +1 for CALL opcode
    code[print_str_patch] = (print_str & 0xFF) as u8;
    code[print_str_patch + 1] = (print_str >> 8) as u8;

    // Repl loop is at repl_loop
    // LD HL, prompt (3 bytes)
    let prompt_patch = repl_loop as usize + 1;
    code[prompt_patch] = (prompt_str & 0xFF) as u8;
    code[prompt_patch + 1] = (prompt_str >> 8) as u8;
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
