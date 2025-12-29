# bc80

An arbitrary-precision decimal calculator and programming language for the Z80 processor. Compiles bc-style programs to native Z80 machine code using BCD (Binary Coded Decimal) arithmetic for exact decimal calculations.

## Features

- **Arbitrary Precision**: Up to 50 decimal digits of precision
- **bc-Compatible Syntax**: Familiar syntax for anyone who has used the Unix `bc` calculator
- **Native Z80 Code**: Compiles directly to Z80 machine code - no interpreter overhead
- **BCD Arithmetic**: Exact decimal math with no floating-point rounding errors
- **Interactive REPL**: Generate a standalone REPL ROM for interactive calculations
- **Signed Numbers**: Full support for negative numbers and signed arithmetic

### Supported Operations

| Operation | Syntax | Example |
|-----------|--------|---------|
| Addition | `a + b` | `1 + 2` |
| Subtraction | `a - b` | `5 - 3` |
| Multiplication | `a * b` | `6 * 7` |
| Division | `a / b` | `22 / 7` |
| Parentheses | `(expr)` | `(1 + 2) * 3` |
| Assignment | `var = expr` | `x = 42` |
| Comparison | `<`, `>`, `<=`, `>=`, `==`, `!=` | `x > 0` |

### Control Structures

```bc
/* If statement */
if (x > 0) {
    x = x - 1
}

/* While loop */
while (n > 0) {
    n = n - 1
}

/* For loop */
for (i = 0; i < 10; i = i + 1) {
    sum = sum + i
}
```

### Functions

```bc
/* Define a function */
define factorial(n) {
    if (n <= 1) return 1
    return n * factorial(n - 1)
}

/* Call it */
factorial(10)
```

### Scale (Decimal Precision)

```bc
/* Set decimal places for division results */
scale = 10

/* Now division produces 10 decimal places */
1 / 3    /* outputs: .3333333333 */

/* Scale also affects decimal multiplication */
scale = 2
2.5 * 2  /* outputs: 5.0 */
```

## Building

Requires Rust 1.70 or later.

```bash
cargo build --release
```

The binary will be at `target/release/bc80`.

## Usage

### Compile a bc Program to Z80 ROM

```bash
bc80 program.bc --rom output.bin
```

### Generate Interactive REPL ROM

```bash
bc80 --repl calculator.bin
```

### Debug Options

```bash
bc80 program.bc --tokens      # Show lexer tokens
bc80 program.bc --ast         # Show parsed AST
bc80 program.bc --bytecode    # Show compiled bytecode
```

## Running on Hardware

The generated ROM images are designed for Z80 systems with:

- **Memory**: ROM at 0x0000, RAM at 0x8000+
- **I/O**: MC6850 ACIA at ports 0x80/0x81 for serial output

### RetroShield Z80

Works with the [RetroShield Z80](https://www.tindie.com/products/8bitforce/retroshield-for-arduino-mega/) on Arduino Mega. Use the included emulator for testing:

```bash
../emulator/retroshield output.bin
```

## Interactive REPL

The REPL (Read-Eval-Print Loop) provides an interactive calculator experience, similar to the traditional Unix `bc` command.

### Generating the REPL ROM

```bash
bc80 --repl calculator.bin
```

This creates a standalone ~2KB ROM that runs an interactive calculator.

### Running the REPL

With the emulator:

```bash
../emulator/retroshield calculator.bin
```

Or load `calculator.bin` onto your RetroShield Z80 hardware.

### REPL Session Example

```
bc80 REPL v1.0
> 2+3
5
> 10*5
50
> scale=2
2
> 7/2
3.50
> 1.5+2.5
4.0
> 100-250
-150
> (1+2)*(3+4)
21
>
```

### REPL Features

| Feature | Example | Description |
|---------|---------|-------------|
| Arithmetic | `2+3*4` | Full operator precedence |
| Decimals | `3.14159` | Enter decimal numbers directly |
| Negative results | `5-10` | Displays `-5` |
| Scale setting | `scale=5` | Set decimal places (echoes the value) |
| Parentheses | `(1+2)*3` | Group expressions |

### REPL Limitations

The REPL is a lightweight implementation optimized for the Z80's limited resources:

- No variables (use the compiled mode for variables)
- No functions (use the compiled mode for `define`)
- No control structures (no `if`, `while`, `for`)
- Expression length limited by input buffer (~80 characters)
- Scale maximum of 50 decimal places

For programs requiring variables, functions, or control structures, write a `.bc` file and compile it with `--rom` instead.

## Examples

### Simple Arithmetic

```bc
/* simple.bc */
1 + 2
3 * 4
10 - 5

a = 42
a

scale = 10
1 / 3
```

### Factorial

```bc
/* factorial.bc */
define f(n) {
    if (n <= 1) return 1
    return n * f(n - 1)
}

f(5)    /* 120 */
f(10)   /* 3628800 */
f(20)   /* 2432902008176640000 */
```

### Calculate Pi (Leibniz Formula)

```bc
/* pi.bc */
scale = 50

define pi(n) {
    auto i, s, t
    s = 0
    t = 1
    for (i = 1; i <= n; i += 2) {
        s = s + t / i
        t = -t
    }
    return 4 * s
}

pi(1000)
```

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Lexer     │────▶│   Parser    │────▶│  Compiler   │────▶│ Z80 Codegen │
│  (lexer.rs) │     │ (parser.rs) │     │(compiler.rs)│     │  (z80.rs)   │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
     │                    │                   │                    │
     ▼                    ▼                   ▼                    ▼
   Tokens               AST              Bytecode            Z80 ROM
```

### BCD Number Format

Numbers are stored in a 28-byte structure:

| Offset | Size | Description |
|--------|------|-------------|
| 0 | 1 | Sign (0x00 = positive, 0x80 = negative) |
| 1 | 1 | Length (always 50 digits) |
| 2 | 1 | Scale (decimal places) |
| 3-27 | 25 | Packed BCD digits (2 digits per byte) |

## Testing

Run the comprehensive math test suite:

```bash
bash tests/math_tests.sh
```

Tests cover:
- Basic integer operations
- Addition, subtraction, multiplication, division
- Negative numbers and signed arithmetic
- Decimal numbers with scale
- Order of operations
- Parentheses
- Variables
- Large numbers (up to 50 digits)

## Limitations

- Maximum 50 decimal digits
- Multiplier limited to 4 digits (0-9999) in current implementation
- No modulo operator (yet)
- No exponentiation operator (yet)
- Single-letter variable names only (a-z)

## License

BSD 3-Clause License. See [LICENSE](LICENSE) for details.

## Author

Alex Jokela

## See Also

- [RetroShield Z80](https://www.tindie.com/products/8bitforce/retroshield-for-arduino-mega/) - Arduino shield for running Z80 code
- [GNU bc](https://www.gnu.org/software/bc/) - The original arbitrary precision calculator
