use std::fmt::{Display, Formatter};
use tokens::Token;

pub mod expr;
pub mod lexer;
pub mod parser;
pub mod tokens;

pub(crate) fn error(token: Option<&Token>, message: &str) {
    if let Some(t) = token {
        eprintln!("[line {}] Error, {}: {}", t.line, t.lexeme, message);
    } else {
        eprintln!("Error: {}", message);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f32),
    String(String),
    Identifier(String),
    Bool(bool),
    Nil,
}

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Number(val) => write!(f, "{}", val),
            Literal::String(val) | Literal::Identifier(val) => write!(f, "{}", val),
            Literal::Bool(val) => write!(f, "{}", val),
            Literal::Nil => write!(f, "Nil"),
        }
    }
}
