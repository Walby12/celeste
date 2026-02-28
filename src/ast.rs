use crate::compiler::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum CelesteType {
    Int,
    String,
    Void,
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
        locals: HashMap<String, Local>,
    },
    Return {
        value: Expr,
    },
    Assign {
        name: String,
        value: Box<Expr>,
    },
}
