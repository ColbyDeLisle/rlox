use crate::{Token, expr::Expr};

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    Block(Vec<Stmt>),
    Class(Token, Vec<Stmt>, Option<Expr>),
    Expression(Expr),
    Var(Token, Option<Expr>),
    Print(Expr),
    Function(Token, Vec<Token>, Vec<Stmt>),
    Return(Token, Option<Expr>),
}
