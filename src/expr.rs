use crate::Literal;
use crate::tokens::Token;
use std::fmt::{Display, Formatter};

#[derive(Debug, PartialEq)]
pub enum Expr {
    Binary(Binary),
    Grouping(Grouping),
    Literal(Literal),
    Unary(Unary),
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Binary(expr) => write!(f, "{}", expr),
            Expr::Grouping(expr) => write!(f, "{}", expr),
            Expr::Literal(literal) => write!(f, "{}", literal),
            Expr::Unary(expr) => write!(f, "{}", expr),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Binary {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

impl Display for Binary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} {} {})", self.operator.lexeme, self.left, self.right)
    }
}

#[derive(Debug, PartialEq)]
pub struct Grouping {
    pub expression: Box<Expr>,
}

impl Display for Grouping {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({})", self.expression)
    }
}

#[derive(Debug, PartialEq)]
pub struct Unary {
    pub operator: Token,
    pub right: Box<Expr>,
}

impl Display for Unary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({} {})", self.operator.lexeme, self.right)
    }
}
