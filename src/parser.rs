use crate::ast::*;
use crate::lexer::{Lexer, TokenInfo};
use crate::token::Token;

pub struct Parser {
    tokens: Vec<TokenInfo>,
    pos: usize,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        Parser {
            tokens: lexer.tokenize(),
            pos: 0,
        }
    }

    fn current(&self) -> &Token {
        &self.tokens.get(self.pos).map(|t| &t.token).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> &Token {
        let tok = self.current().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        self.tokens.get(self.pos - 1).map(|t| &t.token).unwrap_or(&Token::Eof)
    }

    fn expect(&mut self, expected: Token) -> Result<(), String> {
        if self.current() == &expected {
            self.advance();
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, self.current()))
        }
    }

    fn skip_newlines(&mut self) {
        while self.current() == &Token::Newline {
            self.advance();
        }
    }

    fn skip_terminators(&mut self) {
        while matches!(self.current(), Token::Newline | Token::Semicolon) {
            self.advance();
        }
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut functions = Vec::new();
        let mut statements = Vec::new();

        self.skip_newlines();

        while self.current() != &Token::Eof {
            if self.current() == &Token::Define {
                functions.push(self.parse_function()?);
            } else {
                let stmt = self.parse_statement()?;
                if !matches!(stmt, Stmt::Empty) {
                    statements.push(stmt);
                }
            }
            self.skip_terminators();
        }

        Ok(Program { functions, statements })
    }

    fn parse_function(&mut self) -> Result<Function, String> {
        self.expect(Token::Define)?;
        self.skip_newlines();

        let name = match self.current().clone() {
            Token::Ident(n) => {
                self.advance();
                n
            }
            _ => return Err("Expected function name".to_string()),
        };

        self.expect(Token::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(Token::RParen)?;
        self.skip_newlines();

        self.expect(Token::LBrace)?;
        self.skip_newlines();

        // Parse auto declarations
        let mut auto_vars = Vec::new();
        while self.current() == &Token::Auto {
            auto_vars.extend(self.parse_auto()?);
            self.skip_terminators();
        }

        // Parse body
        let mut body = Vec::new();
        while self.current() != &Token::RBrace && self.current() != &Token::Eof {
            let stmt = self.parse_statement()?;
            if !matches!(stmt, Stmt::Empty) {
                body.push(stmt);
            }
            self.skip_terminators();
        }

        self.expect(Token::RBrace)?;

        Ok(Function {
            name,
            params,
            auto_vars,
            body,
        })
    }

    fn parse_param_list(&mut self) -> Result<Vec<FuncParam>, String> {
        let mut params = Vec::new();

        if self.current() == &Token::RParen {
            return Ok(params);
        }

        loop {
            let name = match self.current().clone() {
                Token::Ident(n) => {
                    self.advance();
                    n
                }
                _ => return Err("Expected parameter name".to_string()),
            };

            let is_array = if self.current() == &Token::LBracket {
                self.advance();
                self.expect(Token::RBracket)?;
                true
            } else {
                false
            };

            params.push(FuncParam { name, is_array });

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(params)
    }

    fn parse_auto(&mut self) -> Result<Vec<AutoVar>, String> {
        self.expect(Token::Auto)?;
        let mut vars = Vec::new();

        loop {
            let name = match self.current().clone() {
                Token::Ident(n) => {
                    self.advance();
                    n
                }
                _ => return Err("Expected variable name".to_string()),
            };

            let is_array = if self.current() == &Token::LBracket {
                self.advance();
                self.expect(Token::RBracket)?;
                true
            } else {
                false
            };

            vars.push(AutoVar { name, is_array });

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(vars)
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        self.skip_newlines();

        match self.current().clone() {
            Token::Newline | Token::Semicolon => {
                self.advance();
                Ok(Stmt::Empty)
            }

            Token::LBrace => {
                self.advance();
                self.skip_newlines();
                let mut stmts = Vec::new();
                while self.current() != &Token::RBrace && self.current() != &Token::Eof {
                    let stmt = self.parse_statement()?;
                    if !matches!(stmt, Stmt::Empty) {
                        stmts.push(stmt);
                    }
                    self.skip_terminators();
                }
                self.expect(Token::RBrace)?;
                Ok(Stmt::Block(stmts))
            }

            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::For => self.parse_for(),
            Token::Break => {
                self.advance();
                Ok(Stmt::Break)
            }
            Token::Continue => {
                self.advance();
                Ok(Stmt::Continue)
            }
            Token::Return => self.parse_return(),
            Token::Quit => {
                self.advance();
                Ok(Stmt::Quit)
            }
            Token::Halt => {
                self.advance();
                Ok(Stmt::Halt)
            }
            Token::Print => self.parse_print(),
            Token::Auto => {
                let vars = self.parse_auto()?;
                Ok(Stmt::Auto(vars))
            }

            Token::Eof => Ok(Stmt::Empty),

            _ => {
                let expr = self.parse_expr()?;
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.expect(Token::If)?;
        self.expect(Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Token::RParen)?;
        self.skip_newlines();

        let then_branch = Box::new(self.parse_statement()?);
        self.skip_newlines();

        let else_branch = if self.current() == &Token::Else {
            self.advance();
            self.skip_newlines();
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        Ok(Stmt::If {
            cond,
            then_branch,
            else_branch,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.expect(Token::While)?;
        self.expect(Token::LParen)?;
        let cond = self.parse_expr()?;
        self.expect(Token::RParen)?;
        self.skip_newlines();

        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::While { cond, body })
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.expect(Token::For)?;
        self.expect(Token::LParen)?;

        let init = if self.current() != &Token::Semicolon {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(Token::Semicolon)?;

        let cond = if self.current() != &Token::Semicolon {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(Token::Semicolon)?;

        let update = if self.current() != &Token::RParen {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(Token::RParen)?;
        self.skip_newlines();

        let body = Box::new(self.parse_statement()?);

        Ok(Stmt::For {
            init,
            cond,
            update,
            body,
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Return)?;

        if matches!(self.current(), Token::Newline | Token::Semicolon | Token::RBrace | Token::Eof) {
            Ok(Stmt::Return(None))
        } else if self.current() == &Token::LParen {
            self.advance();
            let expr = self.parse_expr()?;
            self.expect(Token::RParen)?;
            Ok(Stmt::Return(Some(expr)))
        } else {
            let expr = self.parse_expr()?;
            Ok(Stmt::Return(Some(expr)))
        }
    }

    fn parse_print(&mut self) -> Result<Stmt, String> {
        self.expect(Token::Print)?;
        let mut items = Vec::new();

        loop {
            match self.current().clone() {
                Token::String(s) => {
                    self.advance();
                    items.push(PrintItem::String(s));
                }
                Token::Newline | Token::Semicolon | Token::Eof => break,
                _ => {
                    let expr = self.parse_expr()?;
                    items.push(PrintItem::Expr(expr));
                }
            }

            if self.current() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Stmt::Print(items))
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, String> {
        let left = self.parse_or()?;

        match self.current().clone() {
            Token::Assign => {
                self.advance();
                let right = self.parse_assignment()?;
                Ok(Expr::Assign(Box::new(left), Box::new(right)))
            }
            Token::PlusAssign => {
                self.advance();
                let right = self.parse_assignment()?;
                Ok(Expr::AddAssign(Box::new(left), Box::new(right)))
            }
            Token::MinusAssign => {
                self.advance();
                let right = self.parse_assignment()?;
                Ok(Expr::SubAssign(Box::new(left), Box::new(right)))
            }
            Token::StarAssign => {
                self.advance();
                let right = self.parse_assignment()?;
                Ok(Expr::MulAssign(Box::new(left), Box::new(right)))
            }
            Token::SlashAssign => {
                self.advance();
                let right = self.parse_assignment()?;
                Ok(Expr::DivAssign(Box::new(left), Box::new(right)))
            }
            Token::PercentAssign => {
                self.advance();
                let right = self.parse_assignment()?;
                Ok(Expr::ModAssign(Box::new(left), Box::new(right)))
            }
            Token::CaretAssign => {
                self.advance();
                let right = self.parse_assignment()?;
                Ok(Expr::PowAssign(Box::new(left), Box::new(right)))
            }
            _ => Ok(left),
        }
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;

        while self.current() == &Token::Or {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_not()?;

        while self.current() == &Token::And {
            self.advance();
            let right = self.parse_not()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }

        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, String> {
        if self.current() == &Token::Not {
            self.advance();
            let expr = self.parse_not()?;
            Ok(Expr::Not(Box::new(expr)))
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let left = self.parse_additive()?;

        match self.current().clone() {
            Token::Equal => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Eq(Box::new(left), Box::new(right)))
            }
            Token::NotEqual => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Ne(Box::new(left), Box::new(right)))
            }
            Token::Less => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Lt(Box::new(left), Box::new(right)))
            }
            Token::LessEqual => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Le(Box::new(left), Box::new(right)))
            }
            Token::Greater => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Gt(Box::new(left), Box::new(right)))
            }
            Token::GreaterEqual => {
                self.advance();
                let right = self.parse_additive()?;
                Ok(Expr::Ge(Box::new(left), Box::new(right)))
            }
            _ => Ok(left),
        }
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;

        loop {
            match self.current().clone() {
                Token::Plus => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expr::Add(Box::new(left), Box::new(right));
                }
                Token::Minus => {
                    self.advance();
                    let right = self.parse_multiplicative()?;
                    left = Expr::Sub(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_power()?;

        loop {
            match self.current().clone() {
                Token::Star => {
                    self.advance();
                    let right = self.parse_power()?;
                    left = Expr::Mul(Box::new(left), Box::new(right));
                }
                Token::Slash => {
                    self.advance();
                    let right = self.parse_power()?;
                    left = Expr::Div(Box::new(left), Box::new(right));
                }
                Token::Percent => {
                    self.advance();
                    let right = self.parse_power()?;
                    left = Expr::Mod(Box::new(left), Box::new(right));
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let left = self.parse_unary()?;

        if self.current() == &Token::Caret {
            self.advance();
            let right = self.parse_power()?; // Right associative
            Ok(Expr::Pow(Box::new(left), Box::new(right)))
        } else {
            Ok(left)
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.current().clone() {
            Token::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Neg(Box::new(expr)))
            }
            Token::PlusPlus => {
                self.advance();
                let expr = self.parse_postfix()?;
                Ok(Expr::PreInc(Box::new(expr)))
            }
            Token::MinusMinus => {
                self.advance();
                let expr = self.parse_postfix()?;
                Ok(Expr::PreDec(Box::new(expr)))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current().clone() {
                Token::PlusPlus => {
                    self.advance();
                    expr = Expr::PostInc(Box::new(expr));
                }
                Token::MinusMinus => {
                    self.advance();
                    expr = Expr::PostDec(Box::new(expr));
                }
                Token::LBracket => {
                    if let Expr::Var(name) = expr {
                        self.advance();
                        let index = self.parse_expr()?;
                        self.expect(Token::RBracket)?;
                        expr = Expr::ArrayElement(name, Box::new(index));
                    } else {
                        break;
                    }
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.current().clone() {
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }

            Token::String(s) => {
                self.advance();
                Ok(Expr::String(s))
            }

            Token::Scale => {
                self.advance();
                if self.current() == &Token::LParen {
                    self.advance();
                    let expr = self.parse_expr()?;
                    self.expect(Token::RParen)?;
                    Ok(Expr::ScaleFunc(Box::new(expr)))
                } else {
                    Ok(Expr::Scale)
                }
            }

            Token::Ibase => {
                self.advance();
                Ok(Expr::Ibase)
            }

            Token::Obase => {
                self.advance();
                Ok(Expr::Obase)
            }

            Token::Last => {
                self.advance();
                Ok(Expr::Last)
            }

            Token::Length => {
                self.advance();
                self.expect(Token::LParen)?;
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(Expr::Length(Box::new(expr)))
            }

            Token::Sqrt => {
                self.advance();
                self.expect(Token::LParen)?;
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(Expr::Sqrt(Box::new(expr)))
            }

            Token::Read => {
                self.advance();
                self.expect(Token::LParen)?;
                self.expect(Token::RParen)?;
                Ok(Expr::Read)
            }

            Token::Ident(name) => {
                self.advance();
                if self.current() == &Token::LParen {
                    // Function call
                    self.advance();
                    let mut args = Vec::new();
                    if self.current() != &Token::RParen {
                        loop {
                            args.push(self.parse_expr()?);
                            if self.current() == &Token::Comma {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expr::Call(name, args))
                } else if self.current() == &Token::LBracket {
                    // Array element
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(Token::RBracket)?;
                    Ok(Expr::ArrayElement(name, Box::new(index)))
                } else {
                    Ok(Expr::Var(name))
                }
            }

            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }

            _ => Err(format!("Unexpected token: {:?}", self.current())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_expr() {
        let mut parser = Parser::new("1 + 2");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn test_assignment() {
        let mut parser = Parser::new("a = 5");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
    }

    #[test]
    fn test_function() {
        let mut parser = Parser::new("define f(x) { return x * 2 }");
        let program = parser.parse().unwrap();
        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "f");
    }

    #[test]
    fn test_while_loop() {
        let mut parser = Parser::new("while (i < 10) { i = i + 1 }");
        let program = parser.parse().unwrap();
        assert_eq!(program.statements.len(), 1);
    }
}
