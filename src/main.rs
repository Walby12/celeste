mod ast;
mod compiler;
mod lexer;
mod parser;
mod tokens;

use crate::compiler::*;
use crate::tokens::*;

fn main() {
    let mut comp = Compiler::new("fn main() { let x = 12; }".to_string());
    let program = parser::parse(&mut comp);
    for stmt in program.stmts {
        println!("{:?}", stmt);
    }
}
