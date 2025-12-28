mod ast;
mod bytecode;
mod compiler;
mod lexer;
mod parser;
mod token;
mod z80;

use compiler::Compiler;
use std::env;
use std::fs;
use std::process;

fn print_usage(program: &str) {
    eprintln!("bc80 - Arbitrary-precision calculator for Z80");
    eprintln!();
    eprintln!("Usage: {} [options] <file.bc>", program);
    eprintln!("       {} --repl FILE   Generate standalone REPL ROM", program);
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --tokens     Show tokenized output");
    eprintln!("  --ast        Show parsed AST");
    eprintln!("  --bytecode   Show compiled bytecode");
    eprintln!("  --rom FILE   Generate Z80 ROM image");
    eprintln!("  --repl FILE  Generate standalone REPL ROM (no input file needed)");
    eprintln!("  -o FILE      Output file (default: stdout for bytecode)");
    eprintln!("  -h, --help   Show this help");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    let mut show_tokens = false;
    let mut show_ast = false;
    let mut show_bytecode = false;
    let mut rom_file: Option<String> = None;
    let mut repl_file: Option<String> = None;
    let mut output_file: Option<String> = None;
    let mut input_file: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--tokens" => show_tokens = true,
            "--ast" => show_ast = true,
            "--bytecode" => show_bytecode = true,
            "--rom" => {
                i += 1;
                if i < args.len() {
                    rom_file = Some(args[i].clone());
                } else {
                    eprintln!("Error: --rom requires a filename");
                    process::exit(1);
                }
            }
            "--repl" => {
                i += 1;
                if i < args.len() {
                    repl_file = Some(args[i].clone());
                } else {
                    eprintln!("Error: --repl requires a filename");
                    process::exit(1);
                }
            }
            "-o" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(args[i].clone());
                } else {
                    eprintln!("Error: -o requires a filename");
                    process::exit(1);
                }
            }
            "-h" | "--help" => {
                print_usage(&args[0]);
                process::exit(0);
            }
            arg if arg.starts_with('-') => {
                eprintln!("Unknown option: {}", arg);
                process::exit(1);
            }
            _ => {
                if input_file.is_none() {
                    input_file = Some(args[i].clone());
                } else {
                    eprintln!("Multiple input files not supported");
                    process::exit(1);
                }
            }
        }
        i += 1;
    }

    // Handle --repl mode (doesn't require input file)
    if let Some(repl_path) = repl_file {
        let rom = z80::generate_repl_rom();
        match fs::write(&repl_path, &rom) {
            Ok(_) => {
                eprintln!("Wrote {} bytes REPL ROM to {}", rom.len(), repl_path);
            }
            Err(e) => {
                eprintln!("Error writing REPL ROM: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    let input_file = match input_file {
        Some(f) => f,
        None => {
            eprintln!("Error: No input file specified");
            process::exit(1);
        }
    };

    let source = match fs::read_to_string(&input_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading {}: {}", input_file, e);
            process::exit(1);
        }
    };

    // Tokenize
    if show_tokens {
        let mut lexer = lexer::Lexer::new(&source);
        let tokens = lexer.tokenize();
        println!("=== Tokens ===");
        for tok in &tokens {
            println!("{:4}:{:2} {:?}", tok.line, tok.col, tok.token);
        }
        if !show_ast && !show_bytecode && rom_file.is_none() {
            return;
        }
    }

    // Parse
    let mut parser = parser::Parser::new(&source);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    if show_ast {
        println!("=== AST ===");
        println!("Functions:");
        for func in &program.functions {
            println!("  {} ({} params)", func.name, func.params.len());
        }
        println!("Statements: {}", program.statements.len());
        for stmt in &program.statements {
            println!("  {:?}", stmt);
        }
        if !show_bytecode && rom_file.is_none() {
            return;
        }
    }

    // Compile
    let module = match Compiler::compile(&source) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Compile error: {}", e);
            process::exit(1);
        }
    };

    if show_bytecode {
        println!("=== Bytecode ===");
        println!("Size: {} bytes", module.bytecode.len());
        println!("Numbers: {}", module.numbers.len());
        println!("Strings: {}", module.strings.len());
        println!();

        let mut offset = 0;
        while offset < module.bytecode.len() {
            let op = module.bytecode[offset];
            print!("{:04X}: {:02X} ", offset, op);

            if let Some(opcode) = bytecode::Op::from_u8(op) {
                print!("{:?}", opcode);

                // Show operands
                match opcode {
                    bytecode::Op::LoadNum | bytecode::Op::LoadStr | bytecode::Op::PrintStr => {
                        if offset + 2 < module.bytecode.len() {
                            let idx = module.bytecode[offset + 1] as u16
                                | ((module.bytecode[offset + 2] as u16) << 8);
                            print!(" #{}", idx);
                            offset += 2;
                        }
                    }
                    bytecode::Op::LoadVar | bytecode::Op::StoreVar |
                    bytecode::Op::LoadArray | bytecode::Op::StoreArray |
                    bytecode::Op::Call => {
                        if offset + 1 < module.bytecode.len() {
                            print!(" @{}", module.bytecode[offset + 1]);
                            offset += 1;
                        }
                    }
                    bytecode::Op::Jump | bytecode::Op::JumpIfZero | bytecode::Op::JumpIfNotZero => {
                        if offset + 2 < module.bytecode.len() {
                            let addr = module.bytecode[offset + 1] as u16
                                | ((module.bytecode[offset + 2] as u16) << 8);
                            print!(" -> {:04X}", addr);
                            offset += 2;
                        }
                    }
                    _ => {}
                }
            } else {
                print!("???");
            }
            println!();
            offset += 1;
        }

        if rom_file.is_none() {
            return;
        }
    }

    // Generate ROM if requested
    if let Some(rom_path) = rom_file {
        let rom = z80::generate_rom(&module);

        match fs::write(&rom_path, &rom) {
            Ok(_) => {
                eprintln!(
                    "Compiled: {} bytes bytecode, {} numbers, {} strings",
                    module.bytecode.len(),
                    module.numbers.len(),
                    module.strings.len()
                );
                eprintln!(
                    "Wrote {} bytes ROM to {} (runtime: {}B, bytecode at 0x1000)",
                    rom.len(),
                    rom_path,
                    0x1000
                );
            }
            Err(e) => {
                eprintln!("Error writing ROM: {}", e);
                process::exit(1);
            }
        }
    } else if let Some(out_path) = output_file {
        // Write just the bytecode
        match fs::write(&out_path, &module.bytecode) {
            Ok(_) => eprintln!("Wrote {} bytes to {}", module.bytecode.len(), out_path),
            Err(e) => {
                eprintln!("Error writing output: {}", e);
                process::exit(1);
            }
        }
    }
}
