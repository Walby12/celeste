use crate::compiler::*;
use crate::tokens::*;

pub fn lexe(comp: &mut Compiler) {
    let char = comp.src.chars().nth(comp.index);

    match char {
        Some(c) => match c {
            ' ' | '\t' | '\r' => {
                comp.index += 1;
                lexe(comp);
            }
            '\n' => {
                comp.line += 1;
                comp.index += 1;
                lexe(comp);
            }
            '=' => {
                comp.index += 1;
                comp.cur_tok = TokenType::Equals;
            }
            '{' => {
                comp.index += 1;
                comp.cur_tok = TokenType::OpenCurly;
            }
            '}' => {
                comp.index += 1;
                comp.cur_tok = TokenType::CloseCurly;
            }
            '(' => {
                comp.index += 1;
                comp.cur_tok = TokenType::OpenParen;
            }
            ')' => {
                comp.index += 1;
                comp.cur_tok = TokenType::CloseParen;
            }
            ';' => {
                comp.index += 1;
                comp.cur_tok = TokenType::Semicolon;
            }
            _ => {
                let start = comp.index;
                while let Some(curr) = comp.src.chars().nth(comp.index) {
                    if curr.is_whitespace() || "{}();=".contains(curr) {
                        break;
                    }
                    comp.index += 1;
                }
                let value = &comp.src[start..comp.index];

                comp.cur_tok = match value {
                    "fn" => TokenType::Fn,
                    "let" => TokenType::Let,
                    _ if value.chars().all(|c| c.is_ascii_digit()) => {
                        let n = value.parse::<i32>().unwrap_or(0);
                        TokenType::Int(n)
                    }
                    _ => TokenType::Ident(value.to_string()),
                };
            }
        },
        None => comp.cur_tok = TokenType::Eof,
    }
}
