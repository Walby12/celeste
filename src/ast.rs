use crate::compiler::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum CelesteType {
    Int,
    String,
    Void,
}

#[derive(Debug)]
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
}

#[derive(Debug)]
pub enum Stmt {
    Let {
        name: String,
        value: Expr,
    },
    Function {
        name: String,
        params: Vec<Param>,
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
    Extern {
        name: String,
        arg_types: Vec<CelesteType>,
        return_type: CelesteType,
    },
    If {
        condition: Expr,
        then_block: Vec<Stmt>,
        else_ifs: Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
    },
    Expression(Expr),
}
