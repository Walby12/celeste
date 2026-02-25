use crate::tokens::*;

pub struct Compiler {
    pub src: String,
    pub line: u64,
    pub index: usize,
    pub cur_tok: TokenType,
}

impl Compiler {
    pub fn new(src: String) -> Self {
        Self {
            src: src,
            line: 1,
            index: 0,
            cur_tok: TokenType::Eof,
        }
    }
}
