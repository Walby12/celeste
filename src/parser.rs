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
            "void" | "int" => fn_type.clone(),
            _ => {
                eprintln!(
                    "error, line {}: unknow return type {:?}",
                    comp.line, comp.cur_tok
                );
                exit(1);
            }
        }
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

    lexe(comp);
    if !matches!(comp.cur_tok, TokenType::OpenCurly) {
        eprintln!("error: expected {{");
        exit(1);
    }

    let mut func = Stmt::Function {
        name: fn_name,
        return_type: fn_return_type,
        body: Vec::new(),
        locals: comp.locals.clone(),
    };
    parse_block(comp, &mut func);

    if let Stmt::Function { ref mut locals, .. } = func {
        *locals = comp.locals.clone();
        comp.locals.clear();
    }

    func
}

fn parse_block(comp: &mut Compiler, func: &mut Stmt) {
    if matches!(comp.cur_tok, TokenType::OpenCurly) {
        lexe(comp);
    }

    if let Stmt::Function { body, .. } = func {
        while comp.cur_tok != TokenType::CloseCurly && comp.cur_tok != TokenType::Eof {
            let stmt = parse_stmt(comp, func);
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

fn parse_return_stmt(comp: &mut Compiler, func: &Stmt) -> Stmt {
    lexe(comp);

    let return_val = if let TokenType::Ident(ref name) = comp.cur_tok {
        name.clone()
    } else if let TokenType::Int(ref num) = comp.cur_tok {
        if let Stmt::Function { return_type, .. } = func {
            if (return_type != "int") {
                eprintln!("error, line {}: expected an integer for an int returning function")
            }
        }
    } else {
        eprintln!(
            "error, line {}: expected a variable or a valid type, got {:?}",
            comp.line, comp.cur_tok
        );
        exit(1);
    };
}
