use crate::ast::*;
use crate::compiler::*;
use crate::lexer::*;
use crate::tokens::*;

use std::process::*;

pub fn parse(comp: &mut Compiler) -> Program {
    let mut stats = Vec::new();
    lexe(comp);

    while comp.cur_tok != TokenType::Eof {
        let stmt = parse_top_level(comp);
        stats.push(stmt);
    }
    Program { stmts: stats }
}

fn parse_top_level(comp: &mut Compiler) -> Stmt {
    match comp.cur_tok {
        TokenType::Fn => parse_fn_decl(comp),
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
            "error, line {}: expected identifier after fn keyword, got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    };

    lexe(comp);
    if !matches!(comp.cur_tok, TokenType::OpenParen) {
        eprintln!(
            "error, line {}: expected '(' after the function name, got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    }

    lexe(comp);
    if !matches!(comp.cur_tok, TokenType::CloseParen) {
        eprintln!(
            "error, line {}: expected ')' after '(', got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    }

    lexe(comp);
    let fn_return_type = if let TokenType::Ident(ref fn_type) = comp.cur_tok {
        match fn_type.as_str() {
            "void" | "int" => {
                let t = fn_type.clone();
                lexe(comp);
                t
            }
            _ => {
                eprintln!(
                    "error, line {}: unknow return type {:?}",
                    comp.line, comp.cur_tok
                );
                exit(1);
            }
        }
    } else if matches!(comp.cur_tok, TokenType::OpenCurly) {
        "void".to_string()
    } else {
        eprintln!(
            "error, line {}: expected return type after ')', got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    };

    if fn_return_type != "int" && fn_name == "main" {
        eprintln!(
            "error, line {}: main expects an int return type, got {}",
            comp.line, fn_return_type
        );
        exit(1);
    }

    if !matches!(comp.cur_tok, TokenType::OpenCurly) {
        eprintln!("error: expected {{");
        exit(1);
    }

    let mut func = Stmt::Function {
        name: fn_name,
        return_type: fn_return_type.clone(),
        body: Vec::new(),
        locals: comp.locals.clone(),
    };
    let had_return = parse_block(comp, &mut func);
    if fn_return_type != "void" && !had_return {
        eprintln!(
            "error, line {}: did not found a return statement in non void function",
            comp.line
        );
        exit(1);
    }

    if let Stmt::Function { ref mut locals, .. } = func {
        *locals = comp.locals.clone();
        comp.locals.clear();
    }

    func
}

fn parse_block(comp: &mut Compiler, func: &mut Stmt) -> bool {
    if matches!(comp.cur_tok, TokenType::OpenCurly) {
        lexe(comp);
    }

    let mut has_return = false;
    let mut stmts = Vec::new();

    while comp.cur_tok != TokenType::CloseCurly && comp.cur_tok != TokenType::Eof {
        let stmt = parse_stmt(comp, func);
        if matches!(stmt, Stmt::Return { .. }) {
            has_return = true;
        }

        stmts.push(stmt);
    }

    if let Stmt::Function { body, .. } = func {
        body.extend(stmts);
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
    has_return
}

fn parse_stmt(comp: &mut Compiler, func: &Stmt) -> Stmt {
    match comp.cur_tok {
        TokenType::Let => parse_let_stmt(comp),
        TokenType::Return => parse_return_stmt(comp, func),
        _ => {
            eprintln!(
                "error line {}: unknown statement in function scope {:?}",
                comp.line, comp.cur_tok
            );
            exit(1);
        }
    }
}

fn parse_let_stmt(comp: &mut Compiler) -> Stmt {
    lexe(comp);

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

    let value_expr = parse_expr(comp);

    if matches!(comp.cur_tok, TokenType::Semicolon) {
        lexe(comp);
    } else {
        eprintln!(
            "error, line {}: expected ';' after let statement, got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    }
    comp.locals.insert(var_name.clone());

    Stmt::Let {
        name: var_name,
        value: value_expr,
    }
}

fn parse_return_stmt(comp: &mut Compiler, func: &Stmt) -> Stmt {
    lexe(comp);

    let is_void = if let Stmt::Function { return_type, .. } = func {
        return_type == "void"
    } else {
        false
    };

    if is_void {
        if !matches!(comp.cur_tok, TokenType::Semicolon) {
            eprintln!(
                "error, line {}: expected an empty return statement for void function",
                comp.line
            );
            exit(1);
        }

        lexe(comp);
        return Stmt::Return {
            value: Expr::Integer(0),
        };
    }

    let return_val = parse_expr(comp);

    if matches!(comp.cur_tok, TokenType::Semicolon) {
        lexe(comp);
    }
    Stmt::Return { value: return_val }
}

fn parse_expr(comp: &mut Compiler) -> Expr {
    parse_additive(comp)
}

fn parse_additive(comp: &mut Compiler) -> Expr {
    let mut lhs = parse_multiplicative(comp);

    while matches!(comp.cur_tok, TokenType::Plus | TokenType::Minus) {
        let op = if matches!(comp.cur_tok, TokenType::Plus) {
            '+'
        } else {
            '-'
        };
        lexe(comp);
        let rhs = parse_multiplicative(comp);
        lhs = Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
    }
    lhs
}

fn parse_multiplicative(comp: &mut Compiler) -> Expr {
    let mut lhs = parse_primary(comp);

    while matches!(comp.cur_tok, TokenType::Star | TokenType::Slash) {
        let op = if matches!(comp.cur_tok, TokenType::Star) {
            '*'
        } else {
            '/'
        };
        lexe(comp);

        let rhs = parse_primary(comp);
        lhs = Expr::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        };
    }
    lhs
}

fn parse_primary(comp: &mut Compiler) -> Expr {
    match comp.cur_tok.clone() {
        TokenType::Int(n) => {
            lexe(comp);
            Expr::Integer(n)
        }
        TokenType::Ident(name) => {
            if !comp.locals.contains(&name) {
                eprintln!("error, line {}: unknown variable '{}'", comp.line, name);
                std::process::exit(1);
            }
            lexe(comp);
            Expr::Variable(name)
        }
        TokenType::OpenParen => {
            lexe(comp);

            let expr = parse_expr(comp);

            if !matches!(comp.cur_tok, TokenType::CloseParen) {
                eprintln!(
                    "error, line {}: expected ')', got {:?}",
                    comp.line, comp.cur_tok
                );
                std::process::exit(1);
            }
            lexe(comp);

            expr
        }
        _ => {
            eprintln!(
                "error, line {}: expected integer or variable, got {:?}",
                comp.line, comp.cur_tok
            );
            std::process::exit(1);
        }
    }
}
