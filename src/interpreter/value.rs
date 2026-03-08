use crate::interpreter::functions::LoxCallable;
use std::fmt::{Display, Formatter};
use std::rc::Rc;

/// A Lox value.
#[derive(Debug, Clone)]
pub enum Value {
    /// A Lox number. All Lox numbers are represented as `f32`s.
    Number(f32),
    /// A Lox string.
    String(String),
    /// A Lox Boolean.
    Bool(bool),
    /// A Lox callable.
    Callable(Rc<dyn LoxCallable>),
    /// The special Lox value, `Nil`.
    Nil,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => *a == *b,
            (Value::String(a), Value::String(b)) => *a == *b,
            (Value::Bool(a), Value::Bool(b)) => *a == *b,
            (Value::Nil, Value::Nil) => true,
            (Value::Callable(f), Value::Callable(g)) => f.name() == g.name(),
            _ => false,
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(
                f,
                "{}",
                format!("{:.3}", n)
                    .trim_end_matches('0')
                    .trim_end_matches('.')
            ),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::Callable(c) => {
                if c.name().is_empty() {
                    write!(f, "<native fn>")
                } else {
                    write!(f, "<fn {}>", c.name())
                }
            }
        }
    }
}

impl Value {
    pub fn to_test_string(&self) -> String {
        match self {
            Value::Number(n) => {
                // If the number is whole, show it as an integer too
                let int_part = *n as i32;
                if (*n - int_part as f32).abs() < f32::EPSILON {
                    // Output: "Number 123 123.0"
                    format!("Number {} {}", int_part, n)
                } else {
                    // Output: "Number 3.14 3.14"
                    format!("Number {} {}", n, n)
                }
            }
            Value::String(s) => format!("String {}", s),
            Value::Bool(b) => format!("Boolean {}", b),
            Value::Nil => "Nil nil".to_string(),
            Value::Callable(c) => {
                // Test suite usually expects something like "<fn foo>" or "<native fn>"
                if c.name().is_empty() {
                    "<native fn>".to_string()
                } else {
                    format!("<fn {}>", c.name())
                }
            }
        }
    }
}
