#[derive(Debug, Clone, PartialEq)]
pub enum CelesteType {
    Int,
    String,
    Void,
    Pointer(Box<CelesteType>),
    Array(Box<CelesteType>),
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: CelesteType,
}

#[derive(Debug)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug)]
pub enum Expr {
    Variable(String),
    Integer(i32),
    StringLiteral(String),
    Binary {
        op: char,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
    Unary {
        op: char,
        right: Box<Expr>,
    },
    ArrayLiteral(Vec<Expr>),
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
    },
    AddressOf(Box<Expr>),
    Deref(Box<Expr>),
}

#[derive(Debug)]
pub enum Stmt {
    Function {
        name: String,
        params: Vec<Param>,
        return_type: String,
        body: Vec<Stmt>,
    },
    Return {
        value: Expr,
        line: usize,
    },
    Let {
        name: String,
        value: Box<Expr>,
        line: usize,
    },
    Assign {
        name: String,
        value: Box<Expr>,
        line: usize,
    },
    Extern {
        name: String,
        arg_types: Vec<CelesteType>,
        return_type: CelesteType,
        is_variadic: bool,
    },
    If {
        condition: Expr,
        then_block: Vec<Stmt>,
        else_ifs: Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
        line: usize,
    },
    For {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        post: Option<Box<Stmt>>,
        body: Vec<Stmt>,
    },
    PtrAssign {
        ptr_expr: Box<Expr>,
        value: Box<Expr>,
    },
    IndexAssign {
        array: Expr,
        index: Expr,
        value: Box<Expr>,
    },
    Expression(Expr, usize),
}
