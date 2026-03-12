use std::cell::RefCell;
use crate::{interpreter::functions::LoxCallable};
use super::class::{Class, Instance};
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
    /// A Lox class.
    Class(Rc<Class>),
    /// An instance of a Lox class.
    Instance(Rc<RefCell<Instance>>),
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
            (Value::Class(c), Value::Class(d)) => c.class_name == d.class_name,
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
            Value::Class(c) => {
                write!(f, "{c}")
            }
            Value::Instance(i) => {
                write!(f, "{}", i.borrow())
            }
        }
    }
}
