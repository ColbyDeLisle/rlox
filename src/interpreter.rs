use crate::{
    Literal, error,
    expr::{Binary, Expr, Unary},
    tokens::TokenType,
};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f32),
    String(String),
    Bool(bool),
    Nil,
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(val) => write!(f, "{}", val),
            Value::String(val) => write!(f, "{}", val),
            Value::Bool(val) => write!(f, "{}", val),
            Value::Nil => write!(f, "Nil"),
        }
    }
}

#[derive(Debug, Default)]
pub struct Interpreter {
    had_runtime_error: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn expr(&mut self, expr: Expr) -> anyhow::Result<Value> {
        match expr {
            Expr::Binary(binary) => self.binary(binary),
            Expr::Grouping(grouping) => self.expr(*grouping.expression),
            Expr::Literal(literal) => self.literal(literal),
            Expr::Unary(unary) => self.unary(unary),
        }
    }

    fn literal(&self, literal: Literal) -> anyhow::Result<Value> {
        let value = match literal {
            Literal::Number(val) => Value::Number(val),
            Literal::String(val) => Value::String(val),
            Literal::Bool(val) => Value::Bool(val),
            Literal::Nil => Value::Nil,
            Literal::Identifier(_) => todo!(), // assuming this will be done later
        };

        Ok(value)
    }

    fn unary(&mut self, unary: Unary) -> anyhow::Result<Value> {
        let right_value = self.expr(*unary.right)?;

        match (&unary.operator.token_type, right_value) {
            (TokenType::Minus, Value::Number(val)) => Ok(Value::Number(-val)),
            (TokenType::Minus, _) => {
                let msg = "Operand must be a number.";
                error(Some(&unary.operator.clone()), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string())
            }
            (TokenType::Bang, value) => Ok(Value::Bool(!self.is_truthy(value))),
            _ => panic!(),
        }
    }

    fn is_truthy(&self, value: Value) -> bool {
        match value {
            Value::Bool(val) => val,
            Value::Nil => false,
            _ => true,
        }
    }

    fn binary(&mut self, binary: Binary) -> anyhow::Result<Value> {
        let left_value = self.expr(*binary.left)?;
        let right_value = self.expr(*binary.right)?;

        let value = match (&binary.operator.token_type, &left_value, &right_value) {
            (TokenType::Minus, Value::Number(left), Value::Number(right)) => {
                Value::Number(left - right)
            }
            (TokenType::Slash, Value::Number(left), Value::Number(right)) => {
                Value::Number(left / right)
            }
            (TokenType::Star, Value::Number(left), Value::Number(right)) => {
                Value::Number(left * right)
            }
            (TokenType::Plus, Value::Number(left), Value::Number(right)) => {
                Value::Number(left + right)
            }
            (TokenType::Plus, Value::String(left), Value::String(right)) => {
                Value::String(format!("{left}{right}"))
            }
            (TokenType::Greater, Value::Number(left), Value::Number(right)) => {
                Value::Bool(left > right)
            }
            (TokenType::GreaterEqual, Value::Number(left), Value::Number(right)) => {
                Value::Bool(left >= right)
            }
            (TokenType::Less, Value::Number(left), Value::Number(right)) => {
                Value::Bool(left < right)
            }
            (TokenType::LessEqual, Value::Number(left), Value::Number(right)) => {
                Value::Bool(left <= right)
            }
            (TokenType::BangEqual, Value::Number(left), Value::Number(right)) => {
                Value::Bool(*left != *right)
            }
            (TokenType::EqualEqual, Value::Number(left), Value::Number(right)) => {
                Value::Bool(*left == *right)
            }
            (
                TokenType::Minus
                | TokenType::Slash
                | TokenType::Star
                | TokenType::Greater
                | TokenType::GreaterEqual
                | TokenType::Less
                | TokenType::LessEqual
                | TokenType::BangEqual
                | TokenType::EqualEqual,
                _,
                _,
            ) => {
                let msg = "Operands must be numbers.";
                error(Some(&binary.operator.clone()), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string());
            }
            (TokenType::Plus, _, _) => {
                let msg = "Operands must be numbers or strings.";
                error(Some(&binary.operator.clone()), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string())
            }
            _ => panic!(),
        };

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Literal,
        interpreter::{Interpreter, Value},
        lexer::Lexer,
        parser::Parser,
    };

    #[test]
    fn test_interpret_literal() {
        let interpreter = Interpreter::new();

        assert_eq!(
            interpreter.literal(Literal::Number(17.8)).unwrap(),
            Value::Number(17.8)
        );
        assert_eq!(
            interpreter
                .literal(Literal::String(String::from("abcd")))
                .unwrap(),
            Value::String(String::from("abcd"))
        );
        assert_eq!(
            interpreter.literal(Literal::Bool(true)).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(interpreter.literal(Literal::Nil).unwrap(), Value::Nil);
    }

    #[test]
    fn test_interpret_expr() {
        let source = "-(1 + 2) == ((2 * 2) + 5) / -3.0";
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).unwrap();
        let mut interpreter = Interpreter::new();

        assert_eq!(
            interpreter.expr(parser.parse().unwrap()).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn test_interpret_expr_err() {
        let source = "(1 + 2) == ((2 * !false) + 5) / 3.0";
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).unwrap();
        let mut interpreter = Interpreter::new();

        assert!(interpreter.expr(parser.parse().unwrap()).is_err());
    }
}
