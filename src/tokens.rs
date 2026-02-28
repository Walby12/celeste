#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    Ident(String),
    Int(i32),
    Equals,
    OpenCurly,
    CloseCurly,
    OpenParen,
    CloseParen,
    Semicolon,
    Fn,
    Let,
    Return,
    Plus,
    Minus,
    Star,
    Slash,
    Eof,
}
