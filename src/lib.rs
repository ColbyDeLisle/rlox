use std::fmt::{Display, Formatter};
use tokens::Token;

pub mod expr;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod resolver;
pub mod stmt;
pub mod tokens;

/// Display a Lox compile-time error to the user.
pub(crate) fn compile_time_error(token: Option<&Token>, message: &str) {
    if let Some(t) = token {
        eprintln!("[line {}] Error at '{}': {message}", t.line, t.lexeme);
    } else {
        eprintln!("{}", message);
    }
}

/// Display a Lox run-time error to the user.
pub(crate) fn runtime_error(token: Option<&Token>, message: &str) {
    if let Some(t) = token {
        eprintln!("{message}");
        eprintln!("[line {}]", t.line)
    } else {
        eprintln!("{}", message);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number(f32),
    String(String),
    Bool(bool),
    Nil,
}

impl Display for Literal {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Literal::Number(val) => write!(f, "{}", val),
            Literal::String(val) => write!(f, "{}", val),
            Literal::Bool(val) => write!(f, "{}", val),
            Literal::Nil => write!(f, "Nil"),
        }
    }
}
