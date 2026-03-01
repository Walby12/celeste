use crate::ast::*;
use crate::tokens::*;
use cranelift::prelude::Variable;

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Local {
    pub ty: CelesteType,
    pub is_mutable: bool,
}

#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub stack_slot: Option<cranelift::codegen::ir::StackSlot>,
    pub var_type: CelesteType,
    pub is_mutable: bool,
    pub cranelift_var: Option<Variable>,
}

pub struct Compiler {
    pub src: Vec<u8>,
    pub line: usize,
    pub index: usize,
    pub cur_tok: TokenType,
    pub filename: String,
    pub locals: HashMap<String, Local>,
    pub globals: HashMap<String, CelesteType>,
    pub scopes: Vec<HashMap<String, VariableInfo>>,
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
            scopes: Vec::new(),
        }
    }
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn add_variable(&mut self, name: String, info: VariableInfo) {
        if let Some(current_scope) = self.scopes.last_mut() {
            current_scope.insert(name, info);
        }
    }

    pub fn lookup_variable(&self, name: &str) -> Option<&VariableInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info);
            }
        }
        None
    }
    pub fn lookup_variable_mut(&mut self, name: &str) -> Option<&mut VariableInfo> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                return Some(info);
            }
        }
        None
    }
}
