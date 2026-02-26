mod ast;
mod codegen;
mod compiler;
mod lexer;
mod parser;
mod tokens;

use crate::codegen::*;
use crate::compiler::*;

fn main() {
    let mut backend = CraneliftAOTBackend::new();
    let mut comp = Compiler::new("fn main() { let x = y; let y = 13; }".to_string());
    let program = parser::parse(&mut comp);
    for stmt in &program.stmts {
        println!("{:?}", stmt);
    }
    backend.compile_program(&program);
    backend.finalize_to_file("output.obj");
}
