#[derive(Debug, PartialEq)]
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
    Eof,
}
