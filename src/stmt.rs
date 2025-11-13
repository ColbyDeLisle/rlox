use crate::{Token, expr::Expr};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Block(Vec<Stmt>),
    Expression(Expr),
    Print(Expr),
    Var(Token, Option<Expr>),
}

impl Display for Stmt {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Stmt::Block(stmts) => {
                writeln!(f, "{{")?;
                for stmt in stmts {
                    writeln!(f, "{stmt}")?;
                }
                write!(f, "}}")
            }
            Stmt::Expression(expr) => write!(f, "{expr};"),
            Stmt::Print(expr) => write!(f, "print {expr};"),
            Stmt::Var(token, None) => write!(f, "var {};", token.lexeme),
            Stmt::Var(token, Some(initializer)) => {
                write!(f, "var {} = {};", token.lexeme, initializer)
            }
        }
    }
}
