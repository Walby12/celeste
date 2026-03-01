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
        eprintln!("error line {}: expected string after include", comp.line);
        exit(1);
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
            eprintln!("error: could not find file '{}'", include_path_str);
            exit(1);
        }
    }

    let content = std::fs::read_to_string(&full_path).unwrap();
    let mut sub_compiler = Compiler::new(content, &full_path);
    let prog = parse(&mut sub_compiler);

    for (name, ty) in sub_compiler.globals {
        comp.globals.insert(name, ty);
    }

    prog.stmts
}

fn parse_top_level(comp: &mut Compiler) -> Stmt {
    match comp.cur_tok {
        TokenType::Fn => parse_fn_decl(comp),
        TokenType::Extrn => parse_extrn_decl(comp),
        _ => {
            eprintln!(
                "error line {}: unexpected top level token {:?}",
                comp.line, comp.cur_tok
            );
            exit(1);
        }
    }
}

fn parse_fn_decl(comp: &mut Compiler) -> Stmt {
    lexe(comp);

    let fn_name = if let TokenType::Ident(ref name) = comp.cur_tok {
        name.clone()
    } else {
        eprintln!("error line {}: expected function name", comp.line);
        exit(1);
    };
    lexe(comp);

    if !matches!(comp.cur_tok, TokenType::OpenParen) {
        eprintln!("error line {}: expected '('", comp.line);
        exit(1);
    }
    lexe(comp);

    comp.enter_scope();
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
        comp.add_variable(
            p_name,
            VariableInfo {
                var_type: p_ty,
                is_mutable: false,
                stack_slot: None,
                cranelift_var: None,
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

    let body = parse_block_internal(comp, &fn_name, &fn_return_type_str);
    if fn_return_type_str != "void" {
        let has_return = body.iter().any(|stmt| matches!(stmt, Stmt::Return { .. }));

        if let Some(last_stmt) = body.last() {
            if !matches!(last_stmt, Stmt::Return { .. }) {
                eprintln!(
                    "error line {}: function '{}' must end with a return statement",
                    comp.line, fn_name
                );
                std::process::exit(1);
            }
        } else {
            eprintln!(
                "error line {}: function '{}' is empty but expects a return",
                comp.line, fn_name
            );
            std::process::exit(1);
        }
    }
    comp.exit_scope();

    Stmt::Function {
        name: fn_name,
        params,
        return_type: fn_return_type_str,
        body,
        locals: std::collections::HashMap::new(),
    }
}

fn parse_block_internal(comp: &mut Compiler, fn_name: &str, ret_ty: &str) -> Vec<Stmt> {
    if !matches!(comp.cur_tok, TokenType::OpenCurly) {
        eprintln!("error line {}: expected '{{'", comp.line);
        exit(1);
    }
    lexe(comp);

    let mut stmts = Vec::new();
    let func_dummy = Stmt::Function {
        name: fn_name.to_string(),
        params: vec![],
        return_type: ret_ty.to_string(),
        body: vec![],
        locals: std::collections::HashMap::new(),
    };

    while comp.cur_tok != TokenType::CloseCurly && comp.cur_tok != TokenType::Eof {
        stmts.push(parse_stmt(comp, &func_dummy));
    }

    if !matches!(comp.cur_tok, TokenType::CloseCurly) {
        eprintln!("error line {}: expected '}}'", comp.line);
        exit(1);
    }
    lexe(comp);
    stmts
}

fn parse_extrn_decl(comp: &mut Compiler) -> Stmt {
    lexe(comp);
    if !matches!(comp.cur_tok, TokenType::Fn) {
        exit(1);
    }
    lexe(comp);

    let fn_name = if let TokenType::Ident(ref name) = comp.cur_tok {
        name.clone()
    } else {
        exit(1)
    };
    lexe(comp);

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
    }
    comp.globals.insert(fn_name.clone(), return_type.clone());

    Stmt::Extern {
        name: fn_name,
        arg_types,
        return_type,
    }
}

fn parse_stmt(comp: &mut Compiler, func: &Stmt) -> Stmt {
    match comp.cur_tok {
        TokenType::Let => parse_let_stmt(comp),
        TokenType::Return => parse_return_stmt(comp, func),
        TokenType::If => parse_if_stmt(comp, func),
        TokenType::For => parse_for_stmt(comp, func),
        TokenType::While => parse_while_stmt(comp, func),
        TokenType::Ident(_) => {
            let (expr, _) = parse_expr(comp);
            if matches!(comp.cur_tok, TokenType::Equals) {
                if let Expr::Variable(name) = expr {
                    lexe(comp);
                    let (value_expr, _) = parse_expr(comp);
                    let info = comp.lookup_variable(&name).cloned().unwrap_or_else(|| {
                        eprintln!("error line {}: undefined variable '{}'", comp.line, name);
                        exit(1);
                    });
                    if !info.is_mutable {
                        eprintln!(
                            "error line {}: variable '{}' is not mutable",
                            comp.line, name
                        );
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
                    eprintln!("error line {}: invalid assignment target", comp.line);
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
                "error line {}: unknown statement token {:?}",
                comp.line, comp.cur_tok
            );
            exit(1);
        }
    }
}

fn parse_while_stmt(comp: &mut Compiler, func: &Stmt) -> Stmt {
    lexe(comp);

    let has_parens = matches!(comp.cur_tok, TokenType::OpenParen);
    if has_parens {
        lexe(comp);
    }

    let (condition_expr, _) = parse_expr(comp);

    if has_parens {
        if matches!(comp.cur_tok, TokenType::CloseParen) {
            lexe(comp);
        } else {
            eprintln!(
                "error line {}: expected ')' after while condition",
                comp.line
            );
            std::process::exit(1);
        }
    }

    let body = parse_block_as_vec(comp, func);

    Stmt::For {
        init: None,
        condition: Some(condition_expr),
        post: None,
        body,
    }
}

fn parse_for_stmt(comp: &mut Compiler, func: &Stmt) -> Stmt {
    lexe(comp);

    let has_parens = matches!(comp.cur_tok, TokenType::OpenParen);
    if has_parens {
        lexe(comp);
    }

    let mut init = None;
    let mut condition = None;
    let mut post = None;

    if !matches!(comp.cur_tok, TokenType::Semicolon) {
        if matches!(comp.cur_tok, TokenType::Let) {
            init = Some(Box::new(parse_let_stmt(comp)));
        } else {
            let (e, _) = parse_expr(comp);
            init = Some(Box::new(Stmt::Expression(e)));
            if matches!(comp.cur_tok, TokenType::Semicolon) {
                lexe(comp);
            }
        }
    } else {
        lexe(comp);
    }

    if !matches!(comp.cur_tok, TokenType::Semicolon) {
        let (c, _) = parse_expr(comp);
        condition = Some(c);
    }
    if matches!(comp.cur_tok, TokenType::Semicolon) {
        lexe(comp);
    }

    if !matches!(comp.cur_tok, TokenType::CloseParen)
        && !matches!(comp.cur_tok, TokenType::OpenCurly)
    {
        let (expr, _) = parse_expr(comp);

        if matches!(comp.cur_tok, TokenType::Equals) {
            if let Expr::Variable(name) = expr {
                lexe(comp);
                let (val, _) = parse_expr(comp);
                post = Some(Box::new(Stmt::Assign {
                    name,
                    value: Box::new(val),
                }));
            }
        } else {
            post = Some(Box::new(Stmt::Expression(expr)));
        }
    }

    if has_parens && matches!(comp.cur_tok, TokenType::CloseParen) {
        lexe(comp);
    }

    let body = parse_block_as_vec(comp, func);

    Stmt::For {
        init,
        condition,
        post,
        body,
    }
}

fn parse_if_stmt(comp: &mut Compiler, func: &Stmt) -> Stmt {
    lexe(comp);
    if !matches!(comp.cur_tok, TokenType::OpenParen) {
        exit(1);
    }
    lexe(comp);
    let (condition, _) = parse_expr(comp);
    if !matches!(comp.cur_tok, TokenType::CloseParen) {
        exit(1);
    }
    lexe(comp);

    let then_block = parse_block_as_vec(comp, func);
    let mut else_ifs = Vec::new();
    let mut else_block = None;

    while matches!(comp.cur_tok, TokenType::Else) {
        lexe(comp);
        if matches!(comp.cur_tok, TokenType::If) {
            lexe(comp);
            lexe(comp);
            let (ei_cond, _) = parse_expr(comp);
            lexe(comp);
            let ei_body = parse_block_as_vec(comp, func);
            else_ifs.push((ei_cond, ei_body));
        } else {
            else_block = Some(parse_block_as_vec(comp, func));
            break;
        }
    }
    Stmt::If {
        condition,
        then_block,
        else_ifs,
        else_block,
    }
}

fn parse_block_as_vec(comp: &mut Compiler, func: &Stmt) -> Vec<Stmt> {
    if !matches!(comp.cur_tok, TokenType::OpenCurly) {
        eprintln!("error line {}: expected '{{'", comp.line);
        exit(1);
    }
    lexe(comp);
    comp.enter_scope();
    let mut body = Vec::new();
    while comp.cur_tok != TokenType::CloseCurly && comp.cur_tok != TokenType::Eof {
        body.push(parse_stmt(comp, func));
    }
    if !matches!(comp.cur_tok, TokenType::CloseCurly) {
        eprintln!("error line {}: expected '}}'", comp.line);
        exit(1);
    }
    lexe(comp);
    comp.exit_scope();
    body
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
        exit(1)
    };
    lexe(comp);
    if !matches!(comp.cur_tok, TokenType::Equals) {
        eprintln!("error line {}: expected '='", comp.line);
        exit(1);
    }
    lexe(comp);
    let (value_expr, value_type) = parse_expr(comp);
    if matches!(comp.cur_tok, TokenType::Semicolon) {
        lexe(comp);
    }

    comp.add_variable(
        var_name.clone(),
        VariableInfo {
            var_type: value_type,
            is_mutable,
            stack_slot: None,
            cranelift_var: None,
        },
    );
    Stmt::Let {
        name: var_name,
        value: value_expr,
    }
}

fn parse_return_stmt(comp: &mut Compiler, _func: &Stmt) -> Stmt {
    lexe(comp);
    if matches!(comp.cur_tok, TokenType::Semicolon) {
        lexe(comp);
        return Stmt::Return {
            value: Expr::Integer(0),
        };
    }
    let (val_expr, _) = parse_expr(comp);
    if matches!(comp.cur_tok, TokenType::Semicolon) {
        lexe(comp);
    }
    Stmt::Return { value: val_expr }
}

fn parse_expr(comp: &mut Compiler) -> (Expr, CelesteType) {
    parse_comparison(comp)
}

fn parse_comparison(comp: &mut Compiler) -> (Expr, CelesteType) {
    let (mut lhs, mut lhs_ty) = parse_additive(comp);
    while matches!(
        comp.cur_tok,
        TokenType::Less | TokenType::Greater | TokenType::DoubleEquals
    ) {
        let op = match comp.cur_tok {
            TokenType::Less => '<',
            TokenType::Greater => '>',
            TokenType::DoubleEquals => '=',
            _ => break,
        };
        lexe(comp);
        let (rhs, _) = parse_additive(comp);
        lhs = Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
        lhs_ty = CelesteType::Int;
    }
    (lhs, lhs_ty)
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
        let (rhs, _) = parse_multiplicative(comp);
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
        let (rhs, _) = parse_primary(comp);
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
                let ret_ty = comp.globals.get(&name).cloned().unwrap_or(CelesteType::Int);
                (Expr::Call { name, args }, ret_ty)
            } else {
                let info = comp.lookup_variable(&name).cloned().unwrap_or_else(|| {
                    eprintln!("error line {}: undefined variable '{}'", comp.line, name);
                    exit(1);
                });
                (Expr::Variable(name), info.var_type)
            }
        }
        TokenType::OpenParen => {
            lexe(comp);
            let res = parse_expr(comp);
            if !matches!(comp.cur_tok, TokenType::CloseParen) {
                eprintln!("error line {}: expected ')'", comp.line);
                exit(1);
            }
            lexe(comp);
            res
        }
        _ => {
            eprintln!(
                "error line {}: expected expression, got {:?}",
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
