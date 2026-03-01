use crate::ast::*;
use crate::compiler::*;
use std::process::exit;

pub struct TypeChecker<'a> {
    pub comp: &'a mut Compiler,
}

impl<'a> TypeChecker<'a> {
    pub fn new(comp: &'a mut Compiler) -> Self {
        Self { comp }
    }

    fn report_error(&self, message: String, line: usize) -> ! {
        eprintln!(
            "error [file: {}, line: {}]: {}",
            self.comp.filename, line, message
        );
        exit(1);
    }

    pub fn check_program(&mut self, program: &Program) {
        self.comp.register_functions(program);
        for stmt in &program.stmts {
            self.check_stmt(stmt);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Function { body, params, .. } => {
                self.comp.enter_scope();
                for param in params {
                    self.comp.add_variable(
                        param.name.clone(),
                        VariableInfo {
                            var_type: param.ty.clone(),
                            is_mutable: false,
                            cranelift_var: None,
                        },
                    );
                }
                for s in body {
                    self.check_stmt(s);
                }
                self.comp.exit_scope();
            }
            Stmt::Let { name, value, line } => {
                let val_ty = self.check_expr(value, *line);
                self.comp.add_variable(
                    name.clone(),
                    VariableInfo {
                        var_type: val_ty,
                        is_mutable: true,
                        cranelift_var: None,
                    },
                );
            }
            Stmt::Assign { name, value, line } => {
                let var_info = self.comp.lookup_variable(name).cloned().unwrap_or_else(|| {
                    self.report_error(format!("undefined variable '{}'", name), *line)
                });
                let val_ty = self.check_expr(value, *line);
                if var_info.var_type != val_ty {
                    self.report_error(
                        format!(
                            "cannot assign {:?} to variable '{}' of type {:?}",
                            val_ty, name, var_info.var_type
                        ),
                        *line,
                    );
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_ifs,
                else_block,
                line,
            } => {
                self.ensure_int(condition, "if condition", *line);
                self.comp.enter_scope();
                for s in then_block {
                    self.check_stmt(s);
                }
                self.comp.exit_scope();

                for (cond, body) in else_ifs {
                    self.ensure_int(cond, "else if condition", *line);
                    self.comp.enter_scope();
                    for s in body {
                        self.check_stmt(s);
                    }
                    self.comp.exit_scope();
                }
                if let Some(eb) = else_block {
                    self.comp.enter_scope();
                    for s in eb {
                        self.check_stmt(s);
                    }
                    self.comp.exit_scope();
                }
            }
            Stmt::For {
                init,
                condition,
                post,
                body,
            } => {
                self.comp.enter_scope();
                if let Some(i) = init {
                    self.check_stmt(i);
                }
                if let Some(c) = condition {
                    self.ensure_int(c, "for condition", 0);
                }
                if let Some(p) = post {
                    self.check_stmt(p);
                }
                for s in body {
                    self.check_stmt(s);
                }
                self.comp.exit_scope();
            }
            Stmt::Return { value, line } => {
                self.check_expr(value, *line);
            }
            Stmt::Expression(expr, line) => {
                self.check_expr(expr, *line);
            }
            _ => {}
        }
    }

    fn check_expr(&self, expr: &Expr, line: usize) -> CelesteType {
        match expr {
            Expr::Integer(_) => CelesteType::Int,
            Expr::StringLiteral(_) => CelesteType::String,
            Expr::Variable(name) => {
                if let Some(v) = self.comp.lookup_variable(name) {
                    v.var_type.clone()
                } else {
                    self.report_error(format!("undefined variable '{}'", name), line)
                }
            }
            Expr::Binary { op, lhs, rhs } => {
                let lt = self.check_expr(lhs, line);
                let rt = self.check_expr(rhs, line);
                if lt != rt {
                    self.report_error(
                        format!("operator '{}' cannot be used on {:?} and {:?}", op, lt, rt),
                        line,
                    );
                }
                CelesteType::Int
            }
            Expr::Unary { op, right } => {
                let ty = self.check_expr(right, line);
                if *op == '!' && ty != CelesteType::Int {
                    self.report_error(format!("operator '!' cannot be applied to {:?}", ty), line);
                }
                CelesteType::Int
            }
            Expr::Call { name, args } => self.check_call(name, args, line),
            Expr::AddressOf(name) => {
                let info = self.comp.lookup_variable(name).cloned().unwrap_or_else(|| {
                    self.report_error(format!("undefined variable '{}'", name), line)
                });
                CelesteType::Pointer(Box::new(info.var_type))
            }

            Expr::Deref(inner_expr) => {
                let ty = self.check_expr(inner_expr, line);

                if let CelesteType::Pointer(inner_ty) = ty {
                    *inner_ty
                } else {
                    self.report_error("cannot dereference non-pointer type".to_string(), line)
                }
            }
        }
    }

    fn ensure_int(&self, expr: &Expr, context: &str, line: usize) {
        if self.check_expr(expr, line) != CelesteType::Int {
            self.report_error(format!("{} must evaluate to an integer", context), line);
        }
    }

    fn check_call(&self, name: &str, args: &[Expr], line: usize) -> CelesteType {
        if name == "printf" {
            self.check_printf(args, line);
            return CelesteType::Int;
        }

        let fn_info = self.comp.lookup_function(name).unwrap_or_else(|| {
            self.report_error(format!("call to undefined function '{}'", name), line)
        });

        let expected = fn_info.params.len();
        let provided = args.len();

        if fn_info.is_variadic {
            if provided < expected {
                self.report_error(
                    format!(
                        "variadic function '{}' expects at least {} arguments, but only {} were provided",
                        name, expected, provided
                    ),
                    line,
                );
            }
        } else {
            if expected != provided {
                self.report_error(
                    format!(
                        "function '{}' expects {} arguments, but {} were provided",
                        name, expected, provided
                    ),
                    line,
                );
            }
        }

        for (i, (expected_ty, arg_expr)) in fn_info.params.iter().zip(args).enumerate() {
            let provided_ty = self.check_expr(arg_expr, line);
            if expected_ty != &provided_ty {
                self.report_error(
                    format!(
                        "argument {} of function '{}' expects {:?}, but got {:?}",
                        i + 1,
                        name,
                        expected_ty,
                        provided_ty
                    ),
                    line,
                );
            }
        }

        fn_info.return_type.clone()
    }

    fn check_printf(&self, args: &[Expr], line: usize) {
        if args.is_empty() {
            return;
        }

        if let Expr::StringLiteral(fmt) = &args[0] {
            let mut expected = Vec::new();
            let bytes = fmt.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'%' && i + 1 < bytes.len() {
                    match bytes[i + 1] {
                        b'd' => expected.push(CelesteType::Int),
                        b's' => expected.push(CelesteType::String),
                        _ => {}
                    }
                    i += 1;
                }
                i += 1;
            }

            let provided = &args[1..];
            if expected.len() != provided.len() {
                self.report_error(
                    format!(
                        "printf expected {} args for format string, got {}",
                        expected.len(),
                        provided.len()
                    ),
                    line,
                );
            }

            for (idx, exp_ty) in expected.iter().enumerate() {
                let got_ty = self.check_expr(&provided[idx], line);
                if *exp_ty != got_ty {
                    self.report_error(
                        format!(
                            "printf %{} specifier requires {:?}, but got {:?}",
                            if *exp_ty == CelesteType::Int {
                                'd'
                            } else {
                                's'
                            },
                            exp_ty,
                            got_ty
                        ),
                        line,
                    );
                }
            }
        }
    }
}
