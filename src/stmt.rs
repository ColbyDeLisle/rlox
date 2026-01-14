use crate::{Token, expr::Expr};
use std::fmt::{Display, Formatter, write};

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    If(Expr, Box<Stmt>, Option<Box<Stmt>>),
    While(Expr, Box<Stmt>),
    Block(Vec<Stmt>),
    Expression(Expr),
    Var(Token, Option<Expr>),
    Print(Expr),
    Function(Token, Vec<Token>, Vec<Stmt>),
    Return(Token, Option<Expr>),
}

impl Display for Stmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Stmt::If(condition, then_branch, None) => {
                writeln!(f, "if ({condition}) {{")?;
                writeln!(f, "{then_branch}")?;
                writeln!(f, "}}")
            }
            Stmt::If(condition, then_branch, Some(else_branch)) => {
                writeln!(f, "if ({condition}) {{")?;
                writeln!(f, "{then_branch}")?;
                writeln!(f, "}}")?;
                writeln!(f, "else {{")?;
                writeln!(f, "{else_branch}")?;
                writeln!(f, "}}")
            }
            Stmt::While(condition, body) => {
                writeln!(f, "while ({condition}) {{")?;
                writeln!(f, "{body}")?;
                writeln!(f, "}}")
            }
            Stmt::Block(stmts) => {
                writeln!(f, "{{")?;
                for stmt in stmts {
                    writeln!(f, "{stmt}")?;
                }
                writeln!(f, "}}")
            }
            Stmt::Expression(expr) => writeln!(f, "{expr};"),
            Stmt::Var(token, None) => writeln!(f, "var {};", token.lexeme),
            Stmt::Var(token, Some(initializer)) => {
                writeln!(f, "var {} = {};", token.lexeme, initializer)
            }
            Stmt::Print(expr) => writeln!(f, "print {expr};"),
            Stmt::Function(name, params, body) => {
                writeln!(f, "fun {:?}({:?}) {{", name, params)?;
                for stmt in body {
                    writeln!(f, "{stmt}")?;
                }
                writeln!(f, "}}")
            }
            Stmt::Return(keyword, expr) => {
                if let Some(value) = expr {
                    writeln!(f, "return {value};")
                } else {
                    writeln!(f, "return;")
                }
            }
        }
    }
}
