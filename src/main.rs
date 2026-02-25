mod ast;
mod compiler;
mod lexer;
mod parser;
mod tokens;

use crate::compiler::*;
use crate::tokens::*;

fn main() {
    let mut comp = Compiler::new("let x = Hello;".to_string());
    let program = parser::parse(&mut comp);
    for stmt in program.stmts {
        println!("{:?}", stmt);
    }
}
