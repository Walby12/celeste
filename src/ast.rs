#[derive(Debug)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug)]
pub enum Expr {
    Variable(String),
}

#[derive(Debug)]
pub enum Stmt {
    Let { name: String, value: Expr },
    Function { name: String, body: Vec<Stmt> },
    Expression(Expr),
}
