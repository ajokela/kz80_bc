use crate::token::Token;

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub line: usize,
    pub col: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek_ahead(&self, n: usize) -> Option<char> {
        self.input.get(self.pos + n).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else if ch == '\\' && self.peek_ahead(1) == Some('\n') {
                // Line continuation
                self.advance();
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        // bc uses /* */ comments
        if self.peek() == Some('/') && self.peek_ahead(1) == Some('*') {
            self.advance(); // /
            self.advance(); // *
            while let Some(ch) = self.peek() {
                if ch == '*' && self.peek_ahead(1) == Some('/') {
                    self.advance();
                    self.advance();
                    break;
                }
                self.advance();
            }
        }
        // Also # comments (GNU extension)
        if self.peek() == Some('#') {
            while let Some(ch) = self.peek() {
                if ch == '\n' {
                    break;
                }
                self.advance();
            }
        }
    }

    fn read_number(&mut self) -> String {
        let mut num = String::new();

        // Read digits (hex digits allowed if ibase > 10)
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || (ch >= 'A' && ch <= 'F') || ch == '.' {
                num.push(ch);
                self.advance();
            } else if ch == '\\' && self.peek_ahead(1) == Some('\n') {
                // Line continuation in number
                self.advance();
                self.advance();
            } else {
                break;
            }
        }

        num
    }

    fn read_string(&mut self) -> String {
        let mut s = String::new();
        self.advance(); // opening "

        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.advance();
                break;
            } else if ch == '\\' {
                self.advance();
                if let Some(esc) = self.peek() {
                    match esc {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '\\' => s.push('\\'),
                        '"' => s.push('"'),
                        _ => {
                            s.push('\\');
                            s.push(esc);
                        }
                    }
                    self.advance();
                }
            } else {
                s.push(ch);
                self.advance();
            }
        }

        s
    }

    fn read_ident(&mut self) -> String {
        let mut ident = String::new();

        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        ident
    }

    pub fn next_token(&mut self) -> TokenInfo {
        loop {
            self.skip_whitespace();
            self.skip_comment();
            self.skip_whitespace();

            let line = self.line;
            let col = self.col;

            let ch = match self.peek() {
                Some(c) => c,
                None => {
                    return TokenInfo {
                        token: Token::Eof,
                        line,
                        col,
                    }
                }
            };

            let token = match ch {
                '\n' => {
                    self.advance();
                    Token::Newline
                }

                '0'..='9' | '.' if ch == '.' && !self.peek_ahead(1).map_or(false, |c| c.is_ascii_digit()) => {
                    // Just a dot, not a number
                    self.advance();
                    continue; // Ignore stray dots
                }
                '0'..='9' | 'A'..='F' | '.' => {
                    let num = self.read_number();
                    Token::Number(num)
                }

                '"' => {
                    let s = self.read_string();
                    Token::String(s)
                }

                'a'..='z' | '_' | 'G'..='Z' => {
                    let ident = self.read_ident();
                    match ident.as_str() {
                        "if" => Token::If,
                        "else" => Token::Else,
                        "while" => Token::While,
                        "for" => Token::For,
                        "break" => Token::Break,
                        "continue" => Token::Continue,
                        "return" => Token::Return,
                        "define" => Token::Define,
                        "auto" => Token::Auto,
                        "print" => Token::Print,
                        "quit" => Token::Quit,
                        "halt" => Token::Halt,
                        "length" => Token::Length,
                        "scale" => Token::Scale,
                        "sqrt" => Token::Sqrt,
                        "read" => Token::Read,
                        "ibase" => Token::Ibase,
                        "obase" => Token::Obase,
                        "last" => Token::Last,
                        _ => Token::Ident(ident),
                    }
                }

                '+' => {
                    self.advance();
                    if self.peek() == Some('+') {
                        self.advance();
                        Token::PlusPlus
                    } else if self.peek() == Some('=') {
                        self.advance();
                        Token::PlusAssign
                    } else {
                        Token::Plus
                    }
                }

                '-' => {
                    self.advance();
                    if self.peek() == Some('-') {
                        self.advance();
                        Token::MinusMinus
                    } else if self.peek() == Some('=') {
                        self.advance();
                        Token::MinusAssign
                    } else {
                        Token::Minus
                    }
                }

                '*' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::StarAssign
                    } else {
                        Token::Star
                    }
                }

                '/' => {
                    self.advance();
                    if self.peek() == Some('*') {
                        // Comment, go back and skip
                        self.pos -= 1;
                        self.col -= 1;
                        self.skip_comment();
                        continue;
                    } else if self.peek() == Some('=') {
                        self.advance();
                        Token::SlashAssign
                    } else {
                        Token::Slash
                    }
                }

                '%' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::PercentAssign
                    } else {
                        Token::Percent
                    }
                }

                '^' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::CaretAssign
                    } else {
                        Token::Caret
                    }
                }

                '=' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::Equal
                    } else {
                        Token::Assign
                    }
                }

                '!' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::NotEqual
                    } else {
                        Token::Not
                    }
                }

                '<' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::LessEqual
                    } else {
                        Token::Less
                    }
                }

                '>' => {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::GreaterEqual
                    } else {
                        Token::Greater
                    }
                }

                '&' => {
                    self.advance();
                    if self.peek() == Some('&') {
                        self.advance();
                        Token::And
                    } else {
                        continue; // Ignore single &
                    }
                }

                '|' => {
                    self.advance();
                    if self.peek() == Some('|') {
                        self.advance();
                        Token::Or
                    } else {
                        continue; // Ignore single |
                    }
                }

                '(' => {
                    self.advance();
                    Token::LParen
                }
                ')' => {
                    self.advance();
                    Token::RParen
                }
                '{' => {
                    self.advance();
                    Token::LBrace
                }
                '}' => {
                    self.advance();
                    Token::RBrace
                }
                '[' => {
                    self.advance();
                    Token::LBracket
                }
                ']' => {
                    self.advance();
                    Token::RBracket
                }
                ';' => {
                    self.advance();
                    Token::Semicolon
                }
                ',' => {
                    self.advance();
                    Token::Comma
                }

                _ => {
                    self.advance();
                    continue; // Skip unknown characters
                }
            };

            return TokenInfo { token, line, col };
        }
    }

    pub fn tokenize(&mut self) -> Vec<TokenInfo> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.token == Token::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number() {
        let mut lexer = Lexer::new("123.456");
        assert!(matches!(lexer.next_token().token, Token::Number(n) if n == "123.456"));
    }

    #[test]
    fn test_operators() {
        let mut lexer = Lexer::new("+ - * / % ^");
        assert!(matches!(lexer.next_token().token, Token::Plus));
        assert!(matches!(lexer.next_token().token, Token::Minus));
        assert!(matches!(lexer.next_token().token, Token::Star));
        assert!(matches!(lexer.next_token().token, Token::Slash));
        assert!(matches!(lexer.next_token().token, Token::Percent));
        assert!(matches!(lexer.next_token().token, Token::Caret));
    }

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("if else while for define scale sqrt");
        assert!(matches!(lexer.next_token().token, Token::If));
        assert!(matches!(lexer.next_token().token, Token::Else));
        assert!(matches!(lexer.next_token().token, Token::While));
        assert!(matches!(lexer.next_token().token, Token::For));
        assert!(matches!(lexer.next_token().token, Token::Define));
        assert!(matches!(lexer.next_token().token, Token::Scale));
        assert!(matches!(lexer.next_token().token, Token::Sqrt));
    }

    #[test]
    fn test_assignment() {
        let mut lexer = Lexer::new("a = 5");
        assert!(matches!(lexer.next_token().token, Token::Ident(s) if s == "a"));
        assert!(matches!(lexer.next_token().token, Token::Assign));
        assert!(matches!(lexer.next_token().token, Token::Number(n) if n == "5"));
    }
}
