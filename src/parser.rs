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
    if !matches!(comp.cur_tok, TokenType::OpenCurly) {
        eprintln!("error: expected {{");
        exit(1);
    }

    let body = parse_block(comp);

    Stmt::Function {
        name: fn_name,
        body: body,
        locals: comp.locals.clone(),
    }
}

fn parse_block(comp: &mut Compiler) -> Vec<Stmt> {
    if matches!(comp.cur_tok, TokenType::OpenCurly) {
        lexe(comp);
    }

    let mut stmts = Vec::new();
    while comp.cur_tok != TokenType::CloseCurly && comp.cur_tok != TokenType::Eof {
        let stmt = parse_stmt(comp);
        stmts.push(stmt);
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

    stmts
}

fn parse_stmt(comp: &mut Compiler) -> Stmt {
    match comp.cur_tok {
        TokenType::Let => parse_let_stmt(comp),
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

    let value_expr = if let TokenType::Int(ref num) = comp.cur_tok {
        Expr::Integer(num.clone())
    } else if let TokenType::Ident(ref name) = comp.cur_tok {
        Expr::Variable(name.clone())
    } else {
        eprintln!(
            "error, line {}: expected an integer or a variable after '=', got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    };
    lexe(comp);

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
