use crate::ast::*;
use crate::tokens::*;
use cranelift::prelude::Variable;
use cranelift::prelude::types;

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
    pub is_variadic: bool,
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
                        is_variadic: false,
                    };
                    self.functions.insert(name.clone(), info);
                }
                Stmt::Extern {
                    name,
                    arg_types,
                    return_type,
                    is_variadic,
                } => {
                    let info = FunctionInfo {
                        params: arg_types.clone(),
                        return_type: return_type.clone(),
                        is_variadic: *is_variadic,
                    };
                    self.functions.insert(name.clone(), info);
                }
                _ => {}
            }
        }
    }
    pub fn celeste_to_cranelift(&self, ty: &CelesteType) -> types::Type {
        match ty {
            CelesteType::Int => types::I64,
            CelesteType::String => types::I64,
            CelesteType::Pointer(_) => types::I64,
            CelesteType::Void => types::I8,
        }
    }

    pub fn get_expr_type(&self, expr: &Expr) -> CelesteType {
        match expr {
            Expr::Variable(name) => self
                .lookup_variable(name)
                .map(|v| v.var_type.clone())
                .unwrap_or(CelesteType::Int),
            Expr::AddressOf(name) => {
                let inner = self.get_expr_type(&Expr::Variable(name.clone()));
                CelesteType::Pointer(Box::new(inner))
            }
            Expr::Deref(inner) => {
                if let CelesteType::Pointer(base) = self.get_expr_type(inner) {
                    *base
                } else {
                    CelesteType::Int
                }
            }
            Expr::Call { name, .. } => self.globals.get(name).cloned().unwrap_or(CelesteType::Int),
            _ => CelesteType::Int,
        }
    }
}
