use crate::ast::*;
use crate::compiler::*;
use crate::lexer::*;
use crate::tokens::*;

use std::process::*;

pub fn parse(comp: &mut Compiler) -> Program {
    let mut stats = Vec::new();
    lexe(comp);

    while comp.cur_tok != TokenType::Eof {
        let stmt = parse_stmt(comp);
        stats.push(stmt);
    }
    Program { stmts: stats }
}
