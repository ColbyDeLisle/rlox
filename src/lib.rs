use crate::tokens::TokenType;
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
        if t.token_type == TokenType::EOF {
            eprintln!("[line {}] Error at end: {message}", t.line);
        } else {
            eprintln!("[line {}] Error at '{}': {message}", t.line, t.lexeme);
        }
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

#[derive(Debug, Clone)]
pub enum Literal {
    Number(f32),
    String(String),
    Bool(bool),
    Nil,
}

impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Literal::Nil, Literal::Nil) => true,
            (Literal::Bool(a), Literal::Bool(b)) => a == b,
            (Literal::String(a), Literal::String(b)) => a == b,
            (Literal::Number(a), Literal::Number(b)) => (a - b).abs() < f32::EPSILON,
            _ => false,
        }
    }
}

impl Eq for Literal {}

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
