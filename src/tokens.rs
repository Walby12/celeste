#[derive(Debug, PartialEq)]
pub enum TokenType {
    Ident(String),
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
