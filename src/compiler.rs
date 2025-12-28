use crate::ast::*;
use crate::bytecode::*;
use crate::parser::Parser;
use std::collections::HashMap;

pub struct Compiler {
    module: CompiledModule,
    variables: HashMap<String, u8>,
    next_var_slot: u8,
    loop_stack: Vec<LoopContext>,
    functions: HashMap<String, u8>,
}

struct LoopContext {
    break_patches: Vec<usize>,
    continue_target: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            module: CompiledModule::new(),
            variables: HashMap::new(),
            next_var_slot: 0,
            loop_stack: Vec::new(),
            functions: HashMap::new(),
        }
    }

    pub fn compile(source: &str) -> Result<CompiledModule, String> {
        let mut parser = Parser::new(source);
        let program = parser.parse()?;

        let mut compiler = Compiler::new();
        compiler.compile_program(&program)?;

        Ok(compiler.module)
    }

    fn compile_program(&mut self, program: &Program) -> Result<(), String> {
        // First pass: register all functions
        for (i, func) in program.functions.iter().enumerate() {
            self.functions.insert(func.name.clone(), i as u8);
        }

        // Compile main statements
        for stmt in &program.statements {
            self.compile_stmt(stmt)?;
        }

        // Add halt at end of main code
        self.module.emit(Op::Halt);

        // Compile functions
        for func in &program.functions {
            self.compile_function(func)?;
        }

        Ok(())
    }

    fn compile_function(&mut self, func: &Function) -> Result<(), String> {
        let offset = self.module.current_offset();

        // Save current variable state
        let saved_vars = self.variables.clone();
        let saved_next = self.next_var_slot;

        // Add parameters as local variables
        for param in &func.params {
            let slot = self.next_var_slot;
            self.variables.insert(param.name.clone(), slot);
            self.next_var_slot += 1;
        }

        // Add auto variables
        for auto_var in &func.auto_vars {
            let slot = self.next_var_slot;
            self.variables.insert(auto_var.name.clone(), slot);
            self.next_var_slot += 1;
        }

        // Compile body
        for stmt in &func.body {
            self.compile_stmt(stmt)?;
        }

        // Default return 0
        self.module.emit(Op::LoadZero);
        self.module.emit(Op::ReturnValue);

        // Record function info
        self.module.functions.push(CompiledFunction {
            name: func.name.clone(),
            param_count: func.params.len(),
            local_count: func.auto_vars.len(),
            bytecode_offset: offset,
        });

        // Restore variable state
        self.variables = saved_vars;
        self.next_var_slot = saved_next;

        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                // If it's not an assignment, print the result
                if !Self::is_assignment(expr) {
                    self.module.emit(Op::Print);
                    self.module.emit(Op::PrintNewline);
                } else {
                    self.module.emit(Op::Pop);
                }
            }

            Stmt::Print(items) => {
                for item in items {
                    match item {
                        PrintItem::Expr(expr) => {
                            self.compile_expr(expr)?;
                            self.module.emit(Op::Print);
                        }
                        PrintItem::String(s) => {
                            let idx = self.module.add_string(s.clone());
                            self.module.emit(Op::PrintStr);
                            self.module.emit_u16(idx);
                        }
                    }
                }
            }

            Stmt::Block(stmts) => {
                for s in stmts {
                    self.compile_stmt(s)?;
                }
            }

            Stmt::If { cond, then_branch, else_branch } => {
                self.compile_expr(cond)?;

                let else_jump = self.module.current_offset();
                self.module.emit(Op::JumpIfZero);
                self.module.emit_u16(0); // Placeholder

                self.compile_stmt(then_branch)?;

                if let Some(else_branch) = else_branch {
                    let end_jump = self.module.current_offset();
                    self.module.emit(Op::Jump);
                    self.module.emit_u16(0); // Placeholder

                    let else_addr = self.module.current_offset() as u16;
                    self.module.patch_u16(else_jump + 1, else_addr);

                    self.compile_stmt(else_branch)?;

                    let end_addr = self.module.current_offset() as u16;
                    self.module.patch_u16(end_jump + 1, end_addr);
                } else {
                    let end_addr = self.module.current_offset() as u16;
                    self.module.patch_u16(else_jump + 1, end_addr);
                }
            }

            Stmt::While { cond, body } => {
                let loop_start = self.module.current_offset();

                self.loop_stack.push(LoopContext {
                    break_patches: Vec::new(),
                    continue_target: loop_start,
                });

                self.compile_expr(cond)?;

                let exit_jump = self.module.current_offset();
                self.module.emit(Op::JumpIfZero);
                self.module.emit_u16(0); // Placeholder

                self.compile_stmt(body)?;

                self.module.emit(Op::Jump);
                self.module.emit_u16(loop_start as u16);

                let end_addr = self.module.current_offset() as u16;
                self.module.patch_u16(exit_jump + 1, end_addr);

                // Patch break statements
                let ctx = self.loop_stack.pop().unwrap();
                for patch in ctx.break_patches {
                    self.module.patch_u16(patch + 1, end_addr);
                }
            }

            Stmt::For { init, cond, update, body } => {
                // Compile init
                if let Some(init_expr) = init {
                    self.compile_expr(init_expr)?;
                    self.module.emit(Op::Pop);
                }

                let loop_start = self.module.current_offset();
                let update_target = loop_start; // Will be adjusted

                self.loop_stack.push(LoopContext {
                    break_patches: Vec::new(),
                    continue_target: update_target, // Temporary
                });

                // Compile condition
                let exit_jump = if let Some(cond_expr) = cond {
                    self.compile_expr(cond_expr)?;
                    let jump = self.module.current_offset();
                    self.module.emit(Op::JumpIfZero);
                    self.module.emit_u16(0);
                    Some(jump)
                } else {
                    None
                };

                // Compile body
                self.compile_stmt(body)?;

                // Update continue target to point to update section
                let continue_addr = self.module.current_offset();
                if let Some(ctx) = self.loop_stack.last_mut() {
                    ctx.continue_target = continue_addr;
                }

                // Compile update
                if let Some(update_expr) = update {
                    self.compile_expr(update_expr)?;
                    self.module.emit(Op::Pop);
                }

                // Jump back to condition
                self.module.emit(Op::Jump);
                self.module.emit_u16(loop_start as u16);

                let end_addr = self.module.current_offset() as u16;

                // Patch exit jump
                if let Some(jump) = exit_jump {
                    self.module.patch_u16(jump + 1, end_addr);
                }

                // Patch break statements
                let ctx = self.loop_stack.pop().unwrap();
                for patch in ctx.break_patches {
                    self.module.patch_u16(patch + 1, end_addr);
                }
            }

            Stmt::Break => {
                if let Some(ctx) = self.loop_stack.last_mut() {
                    let jump = self.module.current_offset();
                    self.module.emit(Op::Jump);
                    self.module.emit_u16(0); // Placeholder
                    ctx.break_patches.push(jump);
                } else {
                    return Err("break outside loop".to_string());
                }
            }

            Stmt::Continue => {
                if let Some(ctx) = self.loop_stack.last() {
                    self.module.emit(Op::Jump);
                    self.module.emit_u16(ctx.continue_target as u16);
                } else {
                    return Err("continue outside loop".to_string());
                }
            }

            Stmt::Return(expr) => {
                if let Some(e) = expr {
                    self.compile_expr(e)?;
                    self.module.emit(Op::ReturnValue);
                } else {
                    self.module.emit(Op::Return);
                }
            }

            Stmt::Quit | Stmt::Halt => {
                self.module.emit(Op::Halt);
            }

            Stmt::Auto(_) => {
                // Auto declarations are handled at function level
            }

            Stmt::Empty => {}
        }

        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Number(s) => {
                if s == "0" {
                    self.module.emit(Op::LoadZero);
                } else if s == "1" {
                    self.module.emit(Op::LoadOne);
                } else {
                    let num = BcNum::parse(s);
                    let idx = self.module.add_number(num);
                    self.module.emit(Op::LoadNum);
                    self.module.emit_u16(idx);
                }
            }

            Expr::String(s) => {
                let idx = self.module.add_string(s.clone());
                self.module.emit(Op::LoadStr);
                self.module.emit_u16(idx);
            }

            Expr::Var(name) => {
                let slot = self.get_or_create_var(name);
                self.module.emit(Op::LoadVar);
                self.module.emit_u8(slot);
            }

            Expr::ArrayElement(name, index) => {
                let slot = self.get_or_create_var(name);
                self.compile_expr(index)?;
                self.module.emit(Op::LoadArray);
                self.module.emit_u8(slot);
            }

            Expr::Scale => {
                self.module.emit(Op::LoadScale);
            }

            Expr::Ibase => {
                self.module.emit(Op::LoadIbase);
            }

            Expr::Obase => {
                self.module.emit(Op::LoadObase);
            }

            Expr::Last => {
                self.module.emit(Op::LoadLast);
            }

            Expr::Add(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Add);
            }

            Expr::Sub(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Sub);
            }

            Expr::Mul(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Mul);
            }

            Expr::Div(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Div);
            }

            Expr::Mod(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Mod);
            }

            Expr::Pow(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Pow);
            }

            Expr::Neg(a) => {
                self.compile_expr(a)?;
                self.module.emit(Op::Neg);
            }

            Expr::Eq(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Eq);
            }

            Expr::Ne(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Ne);
            }

            Expr::Lt(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Lt);
            }

            Expr::Le(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Le);
            }

            Expr::Gt(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Gt);
            }

            Expr::Ge(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Ge);
            }

            Expr::And(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::And);
            }

            Expr::Or(a, b) => {
                self.compile_expr(a)?;
                self.compile_expr(b)?;
                self.module.emit(Op::Or);
            }

            Expr::Not(a) => {
                self.compile_expr(a)?;
                self.module.emit(Op::Not);
            }

            Expr::PreInc(a) => {
                // ++x: increment and return new value
                self.compile_expr(a)?;
                self.module.emit(Op::Inc);
                self.module.emit(Op::Dup);
                self.compile_store(a)?;
            }

            Expr::PreDec(a) => {
                self.compile_expr(a)?;
                self.module.emit(Op::Dec);
                self.module.emit(Op::Dup);
                self.compile_store(a)?;
            }

            Expr::PostInc(a) => {
                // x++: return old value, then increment
                self.compile_expr(a)?;
                self.module.emit(Op::Dup);
                self.module.emit(Op::Inc);
                self.compile_store(a)?;
            }

            Expr::PostDec(a) => {
                self.compile_expr(a)?;
                self.module.emit(Op::Dup);
                self.module.emit(Op::Dec);
                self.compile_store(a)?;
            }

            Expr::Assign(target, value) => {
                self.compile_expr(value)?;
                self.module.emit(Op::Dup); // Keep value on stack for expression result
                self.compile_store(target)?;
            }

            Expr::AddAssign(target, value) => {
                self.compile_expr(target)?;
                self.compile_expr(value)?;
                self.module.emit(Op::Add);
                self.module.emit(Op::Dup);
                self.compile_store(target)?;
            }

            Expr::SubAssign(target, value) => {
                self.compile_expr(target)?;
                self.compile_expr(value)?;
                self.module.emit(Op::Sub);
                self.module.emit(Op::Dup);
                self.compile_store(target)?;
            }

            Expr::MulAssign(target, value) => {
                self.compile_expr(target)?;
                self.compile_expr(value)?;
                self.module.emit(Op::Mul);
                self.module.emit(Op::Dup);
                self.compile_store(target)?;
            }

            Expr::DivAssign(target, value) => {
                self.compile_expr(target)?;
                self.compile_expr(value)?;
                self.module.emit(Op::Div);
                self.module.emit(Op::Dup);
                self.compile_store(target)?;
            }

            Expr::ModAssign(target, value) => {
                self.compile_expr(target)?;
                self.compile_expr(value)?;
                self.module.emit(Op::Mod);
                self.module.emit(Op::Dup);
                self.compile_store(target)?;
            }

            Expr::PowAssign(target, value) => {
                self.compile_expr(target)?;
                self.compile_expr(value)?;
                self.module.emit(Op::Pow);
                self.module.emit(Op::Dup);
                self.compile_store(target)?;
            }

            Expr::Call(name, args) => {
                // Push arguments
                for arg in args {
                    self.compile_expr(arg)?;
                }

                // Call function
                if let Some(&idx) = self.functions.get(name) {
                    self.module.emit(Op::Call);
                    self.module.emit_u8(idx);
                } else {
                    return Err(format!("Undefined function: {}", name));
                }
            }

            Expr::Length(a) => {
                self.compile_expr(a)?;
                self.module.emit(Op::Length);
            }

            Expr::ScaleFunc(a) => {
                self.compile_expr(a)?;
                self.module.emit(Op::ScaleOf);
            }

            Expr::Sqrt(a) => {
                self.compile_expr(a)?;
                self.module.emit(Op::Sqrt);
            }

            Expr::Read => {
                self.module.emit(Op::Read);
            }
        }

        Ok(())
    }

    fn compile_store(&mut self, target: &Expr) -> Result<(), String> {
        match target {
            Expr::Var(name) => {
                let slot = self.get_or_create_var(name);
                self.module.emit(Op::StoreVar);
                self.module.emit_u8(slot);
            }
            Expr::ArrayElement(name, index) => {
                let slot = self.get_or_create_var(name);
                self.compile_expr(index)?;
                self.module.emit(Op::StoreArray);
                self.module.emit_u8(slot);
            }
            Expr::Scale => {
                self.module.emit(Op::StoreScale);
            }
            Expr::Ibase => {
                self.module.emit(Op::StoreIbase);
            }
            Expr::Obase => {
                self.module.emit(Op::StoreObase);
            }
            _ => return Err("Invalid assignment target".to_string()),
        }
        Ok(())
    }

    fn get_or_create_var(&mut self, name: &str) -> u8 {
        if let Some(&slot) = self.variables.get(name) {
            slot
        } else {
            let slot = self.next_var_slot;
            self.variables.insert(name.to_string(), slot);
            self.next_var_slot += 1;
            slot
        }
    }

    fn is_assignment(expr: &Expr) -> bool {
        matches!(
            expr,
            Expr::Assign(_, _) |
            Expr::AddAssign(_, _) |
            Expr::SubAssign(_, _) |
            Expr::MulAssign(_, _) |
            Expr::DivAssign(_, _) |
            Expr::ModAssign(_, _) |
            Expr::PowAssign(_, _) |
            Expr::PreInc(_) |
            Expr::PreDec(_) |
            Expr::PostInc(_) |
            Expr::PostDec(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_number() {
        let module = Compiler::compile("42").unwrap();
        assert!(module.bytecode.len() > 0);
    }

    #[test]
    fn test_compile_addition() {
        let module = Compiler::compile("1 + 2").unwrap();
        assert!(module.bytecode.contains(&(Op::Add as u8)));
    }

    #[test]
    fn test_compile_variable() {
        let module = Compiler::compile("a = 5").unwrap();
        assert!(module.bytecode.contains(&(Op::StoreVar as u8)));
    }
}
