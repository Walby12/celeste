use crate::ast::*;
use crate::compiler::*;
use crate::lexer::*;
use crate::tokens::*;
use std::path::Path;
use std::process::*;

pub fn parse(comp: &mut Compiler) -> Program {
    let mut stats = Vec::new();
    lexe(comp);

    while comp.cur_tok != TokenType::Eof {
        if matches!(comp.cur_tok, TokenType::Include) {
            let mut included_stmts = parse_include(comp);
            stats.append(&mut included_stmts);
        } else {
            let stmt = parse_top_level(comp);
            stats.push(stmt);
        }
    }
    Program { stmts: stats }
}

fn parse_include(comp: &mut Compiler) -> Vec<Stmt> {
    lexe(comp);
    let include_path_str = if let TokenType::StringLiteral(ref s) = comp.cur_tok {
        s.clone()
    } else {
        std::process::exit(1);
    };
    lexe(comp);
    if matches!(comp.cur_tok, TokenType::Semicolon) {
        lexe(comp);
    }

    let current_file_path = Path::new(&comp.filename);
    let base_dir = current_file_path.parent().unwrap_or(Path::new("."));
    let mut full_path = base_dir.join(&include_path_str);

    if !full_path.exists() {
        let stdlib_path = Path::new("stdlib").join(&include_path_str);
        if stdlib_path.exists() {
            full_path = stdlib_path;
        } else {
            eprintln!(
                "error: could not find file '{}' locally or in stdlib",
                include_path_str
            );
            std::process::exit(1);
        }
    }

    let content = std::fs::read_to_string(&full_path).unwrap();
    let mut sub_compiler = Compiler::new(content, &full_path);
    parse(&mut sub_compiler).stmts
}

fn parse_top_level(comp: &mut Compiler) -> Stmt {
    match comp.cur_tok {
        TokenType::Fn => parse_fn_decl(comp),
        TokenType::Extrn => parse_extrn_decl(comp),
        _ => {
            eprintln!(
                "error, line {}: unexpected statement in top level {:?}",
                comp.line, comp.cur_tok
            );
            exit(1);
        }
    }
}

fn parse_fn_decl(comp: &mut Compiler) -> Stmt {
    lexe(comp);

    comp.locals.clear();

    let fn_name = if let TokenType::Ident(ref name) = comp.cur_tok {
        name.clone()
    } else {
        eprintln!(
            "error, line {}: expected identifier after fn, got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    };
    lexe(comp);

    if !matches!(comp.cur_tok, TokenType::OpenParen) {
        eprintln!(
            "error, line {}: expected '(', got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    }
    lexe(comp);

    let mut params = Vec::new();
    while !matches!(comp.cur_tok, TokenType::CloseParen) {
        let p_name = if let TokenType::Ident(ref name) = comp.cur_tok {
            name.clone()
        } else {
            exit(1)
        };
        lexe(comp);

        let p_ty = if let TokenType::Ident(ref ty_str) = comp.cur_tok {
            string_to_celeste_type(ty_str)
        } else {
            exit(1)
        };
        lexe(comp);

        params.push(Param {
            name: p_name.clone(),
            ty: p_ty.clone(),
        });

        comp.locals.insert(
            p_name,
            Local {
                ty: p_ty,
                is_mutable: false,
            },
        );

        if matches!(comp.cur_tok, TokenType::Comma) {
            lexe(comp);
        }
    }
    lexe(comp);

    let fn_return_type_str = if let TokenType::Ident(ref fn_type) = comp.cur_tok {
        let t = fn_type.clone();
        lexe(comp);
        t
    } else {
        "void".to_string()
    };

    let return_type = string_to_celeste_type(&fn_return_type_str);

    comp.globals.insert(fn_name.clone(), return_type.clone());

    let mut func = Stmt::Function {
        name: fn_name,
        params,
        return_type: fn_return_type_str,
        body: Vec::new(),
        locals: comp.locals.clone(),
    };

    parse_block(comp, &mut func);

    if let Stmt::Function { ref mut locals, .. } = func {
        *locals = comp.locals.clone();
    }

    comp.locals.clear();
    func
}

fn parse_extrn_decl(comp: &mut Compiler) -> Stmt {
    lexe(comp);

    if !matches!(comp.cur_tok, TokenType::Fn) {
        eprintln!("error, line {}: expected 'fn' after 'extern'", comp.line);
        exit(1);
    }
    lexe(comp);
    let fn_name = if let TokenType::Ident(ref name) = comp.cur_tok {
        name.clone()
    } else {
        exit(1);
    };
    lexe(comp);

    if !matches!(comp.cur_tok, TokenType::OpenParen) {
        exit(1);
    }
    lexe(comp);

    let mut arg_types = Vec::new();

    while !matches!(comp.cur_tok, TokenType::CloseParen) {
        if matches!(comp.cur_tok, TokenType::Ellipsis) {
            lexe(comp);
            break;
        }

        if let TokenType::Ident(ref ty_str) = comp.cur_tok {
            arg_types.push(string_to_celeste_type(ty_str));
            lexe(comp);
        }

        if matches!(comp.cur_tok, TokenType::Comma) {
            lexe(comp);
        }
    }

    if !matches!(comp.cur_tok, TokenType::CloseParen) {
        eprintln!("error, line {}: expected ')'", comp.line);
        exit(1);
    }
    lexe(comp);

    let return_type = if let TokenType::Ident(ref ty_str) = comp.cur_tok {
        let t = string_to_celeste_type(ty_str);
        lexe(comp);
        t
    } else {
        CelesteType::Void
    };

    if matches!(comp.cur_tok, TokenType::Semicolon) {
        lexe(comp);
    } else {
        eprintln!(
            "error, line {}: expected ';' after extern declaration",
            comp.line
        );
        exit(1);
    }

    comp.globals.insert(fn_name.clone(), return_type.clone());

    Stmt::Extern {
        name: fn_name,
        arg_types,
        return_type,
    }
}

fn parse_block(comp: &mut Compiler, func: &mut Stmt) {
    if matches!(comp.cur_tok, TokenType::OpenCurly) {
        lexe(comp);
    }

    while comp.cur_tok != TokenType::CloseCurly && comp.cur_tok != TokenType::Eof {
        let stmt = parse_stmt(comp, func);
        if let Stmt::Function { body, .. } = func {
            body.push(stmt);
        }
    }

    if matches!(comp.cur_tok, TokenType::CloseCurly) {
        lexe(comp);
    } else {
        eprintln!(
            "error, line {}: expected '}}' at end of block, got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    }
}

fn parse_stmt(comp: &mut Compiler, func: &Stmt) -> Stmt {
    match comp.cur_tok {
        TokenType::Let => parse_let_stmt(comp),
        TokenType::Return => parse_return_stmt(comp, func),
        TokenType::Ident(_) => {
            let (expr, _) = parse_expr(comp);

            if matches!(comp.cur_tok, TokenType::Equals) {
                if let Expr::Variable(name) = expr {
                    lexe(comp);

                    let (value_expr, _) = parse_expr(comp);

                    let local = comp.locals.get(&name).cloned().unwrap_or_else(|| {
                        eprintln!("error: undefined variable '{}'", name);
                        exit(1);
                    });

                    if !local.is_mutable {
                        eprintln!("error: variable '{}' is not mutable", name);
                        exit(1);
                    }

                    if matches!(comp.cur_tok, TokenType::Semicolon) {
                        lexe(comp);
                    }

                    Stmt::Assign {
                        name,
                        value: Box::new(value_expr),
                    }
                } else {
                    eprintln!("error: left-hand side of assignment must be a variable");
                    exit(1);
                }
            } else {
                if matches!(comp.cur_tok, TokenType::Semicolon) {
                    lexe(comp);
                }
                Stmt::Expression(expr)
            }
        }
        _ => {
            eprintln!(
                "error line {}: unknown statement {:?}",
                comp.line, comp.cur_tok
            );
            exit(1);
        }
    }
}

fn parse_let_stmt(comp: &mut Compiler) -> Stmt {
    lexe(comp);
    let mut is_mutable = false;
    if matches!(comp.cur_tok, TokenType::Mut) {
        is_mutable = true;
        lexe(comp);
    }

    let var_name = if let TokenType::Ident(ref name) = comp.cur_tok {
        name.clone()
    } else {
        eprintln!(
            "error, line {}: expected identifier, got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    };

    lexe(comp);
    if !matches!(comp.cur_tok, TokenType::Equals) {
        eprintln!(
            "error, line {}: expected '=', got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    }
    lexe(comp);

    let (value_expr, value_type) = parse_expr(comp);

    if matches!(comp.cur_tok, TokenType::Semicolon) {
        lexe(comp);
    } else {
        eprintln!(
            "error, line {}: expected ';' after let statement, got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    }

    comp.locals.insert(
        var_name.clone(),
        Local {
            ty: value_type,
            is_mutable,
        },
    );

    Stmt::Let {
        name: var_name,
        value: value_expr,
    }
}

fn parse_return_stmt(comp: &mut Compiler, func: &Stmt) -> Stmt {
    let line_num = comp.line;
    lexe(comp);

    let expected_type = if let Stmt::Function { return_type, .. } = func {
        string_to_celeste_type(return_type)
    } else {
        CelesteType::Void
    };

    if matches!(comp.cur_tok, TokenType::Semicolon) {
        if expected_type != CelesteType::Void {
            eprintln!(
                "error, line {}: function expects {:?} return value, but got empty return",
                line_num, expected_type
            );
            exit(1);
        }
        lexe(comp);
        return Stmt::Return {
            value: Expr::Integer(0),
        };
    }

    let (val_expr, actual_type) = parse_expr(comp);

    if actual_type != expected_type {
        eprintln!(
            "error, line {}: type mismatch. Function declared to return {:?}, but returning {:?}",
            line_num, expected_type, actual_type
        );
        exit(1);
    }

    if matches!(comp.cur_tok, TokenType::Semicolon) {
        lexe(comp);
    } else {
        eprintln!(
            "error, line {}: expected ';' after return statement, got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    }

    Stmt::Return { value: val_expr }
}

fn parse_expr(comp: &mut Compiler) -> (Expr, CelesteType) {
    parse_additive(comp)
}

fn parse_additive(comp: &mut Compiler) -> (Expr, CelesteType) {
    let (mut lhs, mut lhs_ty) = parse_multiplicative(comp);

    while matches!(comp.cur_tok, TokenType::Plus | TokenType::Minus) {
        let op = if matches!(comp.cur_tok, TokenType::Plus) {
            '+'
        } else {
            '-'
        };
        lexe(comp);
        let (rhs, rhs_ty) = parse_multiplicative(comp);

        if lhs_ty != CelesteType::Int || rhs_ty != CelesteType::Int {
            eprintln!(
                "error, line {}: Type mismatch. Cannot use '{}' on {:?} and {:?}",
                comp.line, op, lhs_ty, rhs_ty
            );
            exit(1);
        }

        lhs = Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
        lhs_ty = CelesteType::Int;
    }
    (lhs, lhs_ty)
}

fn parse_multiplicative(comp: &mut Compiler) -> (Expr, CelesteType) {
    let (mut lhs, mut lhs_ty) = parse_primary(comp);

    while matches!(comp.cur_tok, TokenType::Star | TokenType::Slash) {
        let op = if matches!(comp.cur_tok, TokenType::Star) {
            '*'
        } else {
            '/'
        };
        lexe(comp);
        let (rhs, rhs_ty) = parse_primary(comp);

        if lhs_ty != CelesteType::Int || rhs_ty != CelesteType::Int {
            eprintln!("error, line {}: Math requires Integers.", comp.line);
            exit(1);
        }

        lhs = Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
        lhs_ty = CelesteType::Int;
    }
    (lhs, lhs_ty)
}

fn parse_primary(comp: &mut Compiler) -> (Expr, CelesteType) {
    match comp.cur_tok.clone() {
        TokenType::Int(n) => {
            lexe(comp);
            (Expr::Integer(n), CelesteType::Int)
        }
        TokenType::StringLiteral(s) => {
            lexe(comp);
            (Expr::StringLiteral(s), CelesteType::String)
        }
        TokenType::Ident(name) => {
            lexe(comp);

            if matches!(comp.cur_tok, TokenType::OpenParen) {
                lexe(comp);
                let mut args = Vec::new();
                while !matches!(comp.cur_tok, TokenType::CloseParen) {
                    let (arg_expr, _) = parse_expr(comp);
                    args.push(arg_expr);
                    if matches!(comp.cur_tok, TokenType::Comma) {
                        lexe(comp);
                    }
                }
                lexe(comp);
                let ty = comp
                    .locals
                    .get(&name)
                    .map(|l| l.ty.clone())
                    .unwrap_or(CelesteType::Int);

                (Expr::Call { name, args }, ty)
            } else {
                let local = comp.locals.get(&name).cloned().unwrap_or_else(|| {
                    eprintln!("error, line {}: undefined variable '{}'", comp.line, name);
                    exit(1);
                });
                (Expr::Variable(name), local.ty)
            }
        }
        TokenType::OpenParen => {
            lexe(comp);
            let res = parse_expr(comp);
            if !matches!(comp.cur_tok, TokenType::CloseParen) {
                eprintln!(
                    "error, line {}: expected ')', got {:?}",
                    comp.line, comp.cur_tok
                );
                exit(1);
            }
            lexe(comp);
            res
        }
        _ => {
            eprintln!(
                "error, line {}: expected integer or variable, got {:?}",
                comp.line, comp.cur_tok
            );
            exit(1);
        }
    }
}

fn string_to_celeste_type(s: &str) -> CelesteType {
    match s.trim() {
        "int" => CelesteType::Int,
        "string" => CelesteType::String,
        _ => CelesteType::Void,
    }
}
