/// Token types for bc language
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Number(String),     // Arbitrary precision number as string
    String(String),     // String literal

    // Identifiers
    Ident(String),      // Variable name (a-z) or function name

    // Keywords
    If,
    Else,
    While,
    For,
    Break,
    Continue,
    Return,
    Define,             // Function definition
    Auto,               // Local variable
    Print,
    Quit,
    Halt,
    Length,             // length(expr)
    Scale,              // scale(expr) or scale variable
    Sqrt,               // sqrt(expr)
    Read,               // read()
    Ibase,              // Input base
    Obase,              // Output base
    Last,               // Last printed value

    // Operators
    Plus,               // +
    Minus,              // -
    Star,               // *
    Slash,              // /
    Percent,            // %
    Caret,              // ^ (power)

    // Assignment operators
    Assign,             // =
    PlusAssign,         // +=
    MinusAssign,        // -=
    StarAssign,         // *=
    SlashAssign,        // /=
    PercentAssign,      // %=
    CaretAssign,        // ^=

    // Increment/Decrement
    PlusPlus,           // ++
    MinusMinus,         // --

    // Comparison
    Equal,              // ==
    NotEqual,           // !=
    Less,               // <
    LessEqual,          // <=
    Greater,            // >
    GreaterEqual,       // >=

    // Logical
    Not,                // !
    And,                // &&
    Or,                 // ||

    // Delimiters
    LParen,             // (
    RParen,             // )
    LBrace,             // {
    RBrace,             // }
    LBracket,           // [
    RBracket,           // ]
    Semicolon,          // ;
    Comma,              // ,
    Newline,            // Significant in bc

    // Special
    Eof,
}

impl Token {
    pub fn is_assignment_op(&self) -> bool {
        matches!(self,
            Token::Assign | Token::PlusAssign | Token::MinusAssign |
            Token::StarAssign | Token::SlashAssign | Token::PercentAssign |
            Token::CaretAssign
        )
    }
}
