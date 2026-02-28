use crate::compiler::*;
use crate::tokens::*;

pub fn lexe(comp: &mut Compiler) {
    while comp.index < comp.src.len() {
        let c = comp.src[comp.index];
        match c {
            b' ' | b'\t' | b'\r' => comp.index += 1,
            b'\n' => {
                comp.line += 1;
                comp.index += 1;
            }
            _ => break,
        }
    }

    if comp.index >= comp.src.len() {
        comp.cur_tok = TokenType::Eof;
        return;
    }

    let c = comp.src[comp.index];

    match c {
        b'=' => {
            comp.index += 1;
            comp.cur_tok = TokenType::Equals;
            return;
        }
        b'{' => {
            comp.index += 1;
            comp.cur_tok = TokenType::OpenCurly;
            return;
        }
        b'}' => {
            comp.index += 1;
            comp.cur_tok = TokenType::CloseCurly;
            return;
        }
        b'(' => {
            comp.index += 1;
            comp.cur_tok = TokenType::OpenParen;
            return;
        }
        b')' => {
            comp.index += 1;
            comp.cur_tok = TokenType::CloseParen;
            return;
        }
        b';' => {
            comp.index += 1;
            comp.cur_tok = TokenType::Semicolon;
            return;
        }
        _ => {}
    }

    let start = comp.index;

    while comp.index < comp.src.len() {
        let curr = comp.src[comp.index];
        if curr.is_ascii_whitespace() || b"{}();=".contains(&curr) {
            break;
        }
        comp.index += 1;
    }

    let slice = &comp.src[start..comp.index];
    let value = std::str::from_utf8(slice).unwrap_or("");

    comp.cur_tok = match value {
        "fn" => TokenType::Fn,
        "let" => TokenType::Let,
        "return" => TokenType::Return,
        _ if value.chars().all(|c| c.is_ascii_digit()) => {
            let n = value.parse::<i32>().unwrap_or(0);
            TokenType::Int(n)
        }
        _ if !value.is_empty()
            && (value.as_bytes()[0].is_ascii_alphabetic() || value.as_bytes()[0] == b'_') =>
        {
            TokenType::Ident(value.to_string())
        }
        _ => {
            eprintln!("error, line {}: invalid token '{}'", comp.line, value);
            std::process::exit(1);
        }
    };
}
