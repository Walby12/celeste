mod compiler;
mod lexer;
mod tokens;

use crate::compiler::*;
use crate::tokens::*;

fn main() {
    let mut comp = Compiler::new("fn main() {}".to_string());
    lexer::lexe(&mut comp);
    while comp.cur_tok != TokenType::Eof {
        println!("{:?}", comp.cur_tok);
        lexer::lexe(&mut comp);
    }
}
