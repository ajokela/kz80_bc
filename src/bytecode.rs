/// Bytecode opcodes for bc VM
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Op {
    // Stack operations
    Halt = 0x00,
    Nop = 0x01,
    Pop = 0x02,
    Dup = 0x03,

    // Constants
    LoadZero = 0x10,        // Push 0
    LoadOne = 0x11,         // Push 1
    LoadNum = 0x12,         // Push number from constant table (index follows)
    LoadStr = 0x13,         // Push string from constant table

    // Variables
    LoadVar = 0x20,         // Load variable (index follows)
    StoreVar = 0x21,        // Store to variable
    LoadArray = 0x22,       // Load array element (array index, then element index on stack)
    StoreArray = 0x23,      // Store to array element

    // Special variables
    LoadScale = 0x28,       // Push current scale
    StoreScale = 0x29,      // Set scale
    LoadIbase = 0x2A,       // Push current ibase
    StoreIbase = 0x2B,      // Set ibase
    LoadObase = 0x2C,       // Push current obase
    StoreObase = 0x2D,      // Set obase
    LoadLast = 0x2E,        // Push last printed value

    // Arithmetic (arbitrary precision BCD)
    Add = 0x30,
    Sub = 0x31,
    Mul = 0x32,
    Div = 0x33,
    Mod = 0x34,
    Pow = 0x35,
    Neg = 0x36,

    // Comparison (returns 0 or 1)
    Eq = 0x40,
    Ne = 0x41,
    Lt = 0x42,
    Le = 0x43,
    Gt = 0x44,
    Ge = 0x45,

    // Logical
    And = 0x48,
    Or = 0x49,
    Not = 0x4A,

    // Increment/Decrement
    Inc = 0x50,             // Increment value on stack
    Dec = 0x51,             // Decrement value on stack

    // Control flow
    Jump = 0x60,            // Unconditional jump (addr follows)
    JumpIfZero = 0x61,      // Jump if top of stack is zero
    JumpIfNotZero = 0x62,   // Jump if top of stack is not zero

    // Functions
    Call = 0x70,            // Call function (function index follows)
    Return = 0x71,          // Return from function
    ReturnValue = 0x72,     // Return with value on stack

    // Built-in functions
    Length = 0x80,          // Get number of digits
    ScaleOf = 0x81,         // Get scale of number
    Sqrt = 0x82,            // Square root

    // I/O
    Print = 0x90,           // Print top of stack
    PrintStr = 0x91,        // Print string (index follows)
    PrintNewline = 0x92,    // Print newline
    Read = 0x93,            // Read number from input
}

impl Op {
    pub fn from_u8(byte: u8) -> Option<Op> {
        match byte {
            0x00 => Some(Op::Halt),
            0x01 => Some(Op::Nop),
            0x02 => Some(Op::Pop),
            0x03 => Some(Op::Dup),

            0x10 => Some(Op::LoadZero),
            0x11 => Some(Op::LoadOne),
            0x12 => Some(Op::LoadNum),
            0x13 => Some(Op::LoadStr),

            0x20 => Some(Op::LoadVar),
            0x21 => Some(Op::StoreVar),
            0x22 => Some(Op::LoadArray),
            0x23 => Some(Op::StoreArray),

            0x28 => Some(Op::LoadScale),
            0x29 => Some(Op::StoreScale),
            0x2A => Some(Op::LoadIbase),
            0x2B => Some(Op::StoreIbase),
            0x2C => Some(Op::LoadObase),
            0x2D => Some(Op::StoreObase),
            0x2E => Some(Op::LoadLast),

            0x30 => Some(Op::Add),
            0x31 => Some(Op::Sub),
            0x32 => Some(Op::Mul),
            0x33 => Some(Op::Div),
            0x34 => Some(Op::Mod),
            0x35 => Some(Op::Pow),
            0x36 => Some(Op::Neg),

            0x40 => Some(Op::Eq),
            0x41 => Some(Op::Ne),
            0x42 => Some(Op::Lt),
            0x43 => Some(Op::Le),
            0x44 => Some(Op::Gt),
            0x45 => Some(Op::Ge),

            0x48 => Some(Op::And),
            0x49 => Some(Op::Or),
            0x4A => Some(Op::Not),

            0x50 => Some(Op::Inc),
            0x51 => Some(Op::Dec),

            0x60 => Some(Op::Jump),
            0x61 => Some(Op::JumpIfZero),
            0x62 => Some(Op::JumpIfNotZero),

            0x70 => Some(Op::Call),
            0x71 => Some(Op::Return),
            0x72 => Some(Op::ReturnValue),

            0x80 => Some(Op::Length),
            0x81 => Some(Op::ScaleOf),
            0x82 => Some(Op::Sqrt),

            0x90 => Some(Op::Print),
            0x91 => Some(Op::PrintStr),
            0x92 => Some(Op::PrintNewline),
            0x93 => Some(Op::Read),

            _ => None,
        }
    }
}

/// A compiled bc number - stored as packed BCD digits
#[derive(Debug, Clone)]
pub struct BcNum {
    pub negative: bool,
    pub integer_digits: Vec<u8>,    // BCD digits before decimal (high to low)
    pub decimal_digits: Vec<u8>,    // BCD digits after decimal
}

#[allow(dead_code)]
impl BcNum {
    pub fn zero() -> Self {
        BcNum {
            negative: false,
            integer_digits: vec![0],
            decimal_digits: Vec::new(),
        }
    }

    pub fn one() -> Self {
        BcNum {
            negative: false,
            integer_digits: vec![1],
            decimal_digits: Vec::new(),
        }
    }

    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        let negative = s.starts_with('-');
        let s = s.trim_start_matches('-').trim_start_matches('+');

        let parts: Vec<&str> = s.split('.').collect();
        let int_part = parts.get(0).unwrap_or(&"0");
        let dec_part = parts.get(1).unwrap_or(&"");

        let integer_digits: Vec<u8> = if int_part.is_empty() {
            vec![0]
        } else {
            int_part
                .chars()
                .filter_map(|c| c.to_digit(10).map(|d| d as u8))
                .collect()
        };

        let decimal_digits: Vec<u8> = dec_part
            .chars()
            .filter_map(|c| c.to_digit(10).map(|d| d as u8))
            .collect();

        // Remove leading zeros from integer part (keep at least one)
        let integer_digits = {
            let mut v = integer_digits;
            while v.len() > 1 && v[0] == 0 {
                v.remove(0);
            }
            v
        };

        BcNum {
            negative: negative && !(integer_digits == vec![0] && decimal_digits.is_empty()),
            integer_digits,
            decimal_digits,
        }
    }

    /// Pack digits into bytes (2 digits per byte) for storage
    /// Format: [sign:1][len:1][scale:1][packed_digits...]
    /// This matches the runtime's expected format
    ///
    /// All numbers are normalized to FIXED_PACKED_BYTES bytes of packed data
    /// to ensure proper alignment during BCD arithmetic operations.
    pub fn to_packed(&self) -> Vec<u8> {
        const FIXED_PACKED_BYTES: usize = 25;  // 50 digits max
        const FIXED_DIGIT_COUNT: usize = FIXED_PACKED_BYTES * 2;

        let mut result = Vec::new();

        // Collect all digits
        let mut all_digits: Vec<u8> = self.integer_digits.clone();
        all_digits.extend(&self.decimal_digits);

        // Pad with leading zeros to reach fixed digit count
        while all_digits.len() < FIXED_DIGIT_COUNT {
            all_digits.insert(0, 0);
        }

        let scale = self.decimal_digits.len();

        // Header: sign (1 byte) + total digit count (1 byte) + scale (1 byte)
        result.push(if self.negative { 0x80 } else { 0x00 });
        result.push(FIXED_DIGIT_COUNT as u8);  // Always 50 digits
        result.push(scale as u8);

        // Pack digits (2 per byte, high nibble first)
        for chunk in all_digits.chunks(2) {
            let high = chunk[0];
            let low = chunk.get(1).copied().unwrap_or(0);
            result.push((high << 4) | low);
        }

        result
    }
}

/// Compiled module
#[derive(Debug)]
pub struct CompiledModule {
    pub bytecode: Vec<u8>,
    pub numbers: Vec<BcNum>,
    pub strings: Vec<String>,
    pub functions: Vec<CompiledFunction>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct CompiledFunction {
    pub name: String,
    pub param_count: usize,
    pub local_count: usize,
    pub bytecode_offset: usize,
}

impl CompiledModule {
    pub fn new() -> Self {
        CompiledModule {
            bytecode: Vec::new(),
            numbers: Vec::new(),
            strings: Vec::new(),
            functions: Vec::new(),
        }
    }

    pub fn add_number(&mut self, num: BcNum) -> u16 {
        let idx = self.numbers.len();
        self.numbers.push(num);
        idx as u16
    }

    pub fn add_string(&mut self, s: String) -> u16 {
        // Check if already exists
        for (i, existing) in self.strings.iter().enumerate() {
            if existing == &s {
                return i as u16;
            }
        }
        let idx = self.strings.len();
        self.strings.push(s);
        idx as u16
    }

    pub fn emit(&mut self, op: Op) {
        self.bytecode.push(op as u8);
    }

    pub fn emit_u8(&mut self, val: u8) {
        self.bytecode.push(val);
    }

    pub fn emit_u16(&mut self, val: u16) {
        self.bytecode.push((val & 0xFF) as u8);
        self.bytecode.push(((val >> 8) & 0xFF) as u8);
    }

    pub fn current_offset(&self) -> usize {
        self.bytecode.len()
    }

    pub fn patch_u16(&mut self, offset: usize, val: u16) {
        self.bytecode[offset] = (val & 0xFF) as u8;
        self.bytecode[offset + 1] = ((val >> 8) & 0xFF) as u8;
    }
}
