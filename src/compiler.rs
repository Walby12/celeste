use crate::ast::*;
use crate::tokens::*;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Local {
    pub ty: CelesteType,
    pub is_mutable: bool,
}

pub struct Compiler {
    pub src: Vec<u8>,
    pub line: usize,
    pub index: usize,
    pub cur_tok: TokenType,
    pub locals: HashMap<String, Local>,
}

impl Compiler {
    pub fn new(src: String) -> Self {
        Self {
            src: src.into_bytes(),
            line: 1,
            index: 0,
            cur_tok: TokenType::Eof,
            locals: HashMap::new(),
        }
    }
}
