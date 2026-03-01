use crate::ast::*;
use crate::tokens::*;
use cranelift::prelude::Variable;

use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct VariableInfo {
    pub var_type: CelesteType,
    pub is_mutable: bool,
    pub cranelift_var: Option<Variable>,
}

pub struct FunctionInfo {
    pub params: Vec<CelesteType>,
    pub return_type: CelesteType,
}

pub struct Compiler {
    pub src: Vec<u8>,
    pub line: usize,
    pub index: usize,
    pub cur_tok: TokenType,
    pub filename: String,
    pub globals: HashMap<String, CelesteType>,
    pub functions: HashMap<String, FunctionInfo>,
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
            globals: HashMap::new(),
            functions: HashMap::new(),
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

    pub fn lookup_function(&self, name: &str) -> Option<&FunctionInfo> {
        self.functions.get(name)
    }

    pub fn register_functions(&mut self, program: &Program) {
        for stmt in &program.stmts {
            match stmt {
                Stmt::Function {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    let info = FunctionInfo {
                        params: params.iter().map(|p| p.ty.clone()).collect(),
                        return_type: if return_type == "int" {
                            CelesteType::Int
                        } else {
                            CelesteType::Void
                        },
                    };
                    self.functions.insert(name.clone(), info);
                }
                Stmt::Extern {
                    name,
                    arg_types,
                    return_type,
                } => {
                    let info = FunctionInfo {
                        params: arg_types.clone(),
                        return_type: return_type.clone(),
                    };
                    self.functions.insert(name.clone(), info);
                }
                _ => {}
            }
        }
    }
}
