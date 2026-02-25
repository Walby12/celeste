#[derive(Debug, PartialEq)]
pub enum TokenType {
    Ident(String),
    OpenCurly,
    CloseCurly,
    OpenParen,
    CloseParen,
    Semicolon,
    Fn,
    Eof,
}
