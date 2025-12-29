/// AST nodes for bc language

#[derive(Debug, Clone)]
pub enum Expr {
    /// Numeric literal (stored as string for arbitrary precision)
    Number(String),

    /// String literal
    String(String),

    /// Variable reference (single letter a-z or longer name)
    Var(String),

    /// Array element access: name[index]
    ArrayElement(String, Box<Expr>),

    /// Special variables
    Scale,
    Ibase,
    Obase,
    Last,

    /// Binary operations
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Mod(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),

    /// Comparison
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Le(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),

    /// Logical
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),

    /// Unary minus
    Neg(Box<Expr>),

    /// Increment/Decrement (returns value before/after)
    PreInc(Box<Expr>),   // ++x
    PreDec(Box<Expr>),   // --x
    PostInc(Box<Expr>),  // x++
    PostDec(Box<Expr>),  // x--

    /// Assignment (returns the assigned value)
    Assign(Box<Expr>, Box<Expr>),
    AddAssign(Box<Expr>, Box<Expr>),
    SubAssign(Box<Expr>, Box<Expr>),
    MulAssign(Box<Expr>, Box<Expr>),
    DivAssign(Box<Expr>, Box<Expr>),
    ModAssign(Box<Expr>, Box<Expr>),
    PowAssign(Box<Expr>, Box<Expr>),

    /// Function call
    Call(String, Vec<Expr>),

    /// Built-in functions
    Length(Box<Expr>),
    ScaleFunc(Box<Expr>),
    Sqrt(Box<Expr>),
    Read,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    /// Expression statement (value is printed if not assignment)
    Expr(Expr),

    /// Print statement: print expr, expr, ...
    Print(Vec<PrintItem>),

    /// Block of statements
    Block(Vec<Stmt>),

    /// If statement
    If {
        cond: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },

    /// While loop
    While {
        cond: Expr,
        body: Box<Stmt>,
    },

    /// For loop: for (init; cond; update) body
    For {
        init: Option<Expr>,
        cond: Option<Expr>,
        update: Option<Expr>,
        body: Box<Stmt>,
    },

    /// Break statement
    Break,

    /// Continue statement
    Continue,

    /// Return statement
    Return(Option<Expr>),

    /// Quit (exit program)
    Quit,

    /// Halt (stop execution)
    Halt,

    /// Auto (local variable declaration)
    #[allow(dead_code)]
    Auto(Vec<AutoVar>),

    /// Empty statement
    Empty,
}

#[derive(Debug, Clone)]
pub enum PrintItem {
    Expr(Expr),
    String(String),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AutoVar {
    pub name: String,
    pub is_array: bool,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<FuncParam>,
    pub auto_vars: Vec<AutoVar>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct FuncParam {
    pub name: String,
    #[allow(dead_code)]
    pub is_array: bool,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
    pub statements: Vec<Stmt>,
}
