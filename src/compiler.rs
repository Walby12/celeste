use crate::ast::*;
use crate::tokens::*;

use std::collections::HashMap;
use std::path::Path;

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
    pub filename: String,
    pub locals: HashMap<String, Local>,
    pub globals: HashMap<String, CelesteType>,
}

impl Compiler {
    pub fn new(src: String, path: &Path) -> Self {
        Self {
            src: src.into_bytes(),
            line: 1,
            index: 0,
            cur_tok: TokenType::Eof,
            filename: path.to_string_lossy().into_owned(),
            locals: HashMap::new(),
            globals: HashMap::new(),
        }
    }
}
