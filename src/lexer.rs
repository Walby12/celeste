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
        b'+' => {
            comp.index += 1;
            comp.cur_tok = TokenType::Plus;
            return;
        }
        b'-' => {
            comp.index += 1;
            comp.cur_tok = TokenType::Minus;
            return;
        }
        b'*' => {
            comp.index += 1;
            comp.cur_tok = TokenType::Star;
            return;
        }
        b'/' => {
            let start_index = comp.index;
            comp.index += 1;

            if comp.index < comp.src.len() && comp.src[comp.index] == b'/' {
                comp.index += 1;
                while comp.index < comp.src.len() && comp.src[comp.index] != b'\n' {
                    comp.index += 1;
                }
                return lexe(comp);
            } else if comp.index < comp.src.len() && comp.src[comp.index] == b'*' {
                comp.index += 1;
                let mut found_end = false;

                while comp.index + 1 < comp.src.len() {
                    if comp.src[comp.index] == b'*' && comp.src[comp.index + 1] == b'/' {
                        comp.index += 2;
                        found_end = true;
                        break;
                    }
                    comp.index += 1;
                }

                if !found_end {
                    eprintln!(
                        "error: unterminated block comment starting at index {}",
                        start_index
                    );
                    std::process::exit(1);
                }

                return lexe(comp);
            } else {
                comp.cur_tok = TokenType::Slash;
                return;
            }
        }
        b'.' => {
            if comp.index + 1 < comp.src.len() && comp.src[comp.index + 1] == b'.' {
                comp.index += 2;
                comp.cur_tok = TokenType::Ellipsis;
                return;
            }
        }
        b',' => {
            comp.index += 1;
            comp.cur_tok = TokenType::Comma;
            return;
        }
        b'"' => {
            comp.index += 1;
            let mut decoded = String::new();
            let mut closed = false;

            while comp.index < comp.src.len() {
                let b = comp.src[comp.index];

                if b == b'"' {
                    closed = true;
                    comp.index += 1;
                    break;
                }

                if b == b'\\' {
                    comp.index += 1;
                    if comp.index < comp.src.len() {
                        match comp.src[comp.index] {
                            b'n' => decoded.push('\n'),
                            b'r' => decoded.push('\r'),
                            b't' => decoded.push('\t'),
                            b'\\' => decoded.push('\\'),
                            b'"' => decoded.push('"'),
                            _ => {
                                decoded.push('\\');
                                decoded.push(comp.src[comp.index] as char);
                            }
                        }
                    }
                } else {
                    decoded.push(b as char);
                }
                comp.index += 1;
            }

            if !closed {
                eprintln!("error, line {}: unclosed string literal", comp.line);
                std::process::exit(1);
            }

            comp.cur_tok = TokenType::StringLiteral(decoded);
            return;
        }
        _ => {}
    }

    let start = comp.index;

    while comp.index < comp.src.len() {
        let curr = comp.src[comp.index];
        if curr.is_ascii_whitespace() || b"{}();=+-*/\",.&".contains(&curr) {
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
        "mut" => TokenType::Mut,
        "extrn" => TokenType::Extrn,
        "include" => TokenType::Include,
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
