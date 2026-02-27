use std::collections::HashSet;

#[derive(Debug)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug)]
pub enum Expr {
    Variable(String),
    Integer(i32),
}

#[derive(Debug)]
pub enum Stmt {
    Let {
        name: String,
        value: Expr,
    },
    Function {
        name: String,
        return_type: String,
        body: Vec<Stmt>,
        locals: HashSet<String>,
    },
    Expression(Expr),
}
