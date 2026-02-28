use crate::tokens::*;
use std::collections::HashSet;

pub struct Compiler {
    pub src: Vec<u8>,
    pub line: usize,
    pub index: usize,
    pub cur_tok: TokenType,
    pub locals: HashSet<String>,
}

impl Compiler {
    pub fn new(src: String) -> Self {
        Self {
            src: src.into_bytes(),
            line: 1,
            index: 0,
            cur_tok: TokenType::Eof,
            locals: HashSet::new(),
        }
    }
}
