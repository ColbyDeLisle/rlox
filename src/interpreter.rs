use crate::{
    Literal, error,
    expr::{Binary, Expr, Unary},
    stmt::Stmt,
    tokens::TokenType,
};
use std::{
    fmt::{Display, Formatter},
    mem::take,
};

mod environment;
use crate::expr::{Assign, Logical};
use environment::Environment;

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
    environment: Environment,
    had_runtime_error: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn interpret(&mut self, stmts: Vec<Stmt>) -> anyhow::Result<()> {
        for stmt in stmts {
            self.stmt(stmt)?;
        }

        match self.had_runtime_error {
            false => Ok(()),
            true => anyhow::bail!(""),
        }
    }

    fn stmt(&mut self, stmt: Stmt) -> anyhow::Result<()> {
        match stmt {
            Stmt::If(condition, then_branch, else_branch) => {
                let condition_value = self.expr(condition)?;

                if self.is_truthy(&condition_value) {
                    self.stmt(*then_branch)?;
                } else if let Some(stmt) = else_branch {
                    self.stmt(*stmt)?;
                }

                Ok(())
            }
            Stmt::While(condition, body) => {
                // Here is another place where using only `Box` for indirection gets ugly

                let mut condition_value = self.expr(condition.clone())?;

                while self.is_truthy(&condition_value) {
                    // Using `Box` causes the need for cloning here.
                    // I think this might be sufficiently bad for me to want to refactor now!
                    self.stmt(*body.clone())?;
                    condition_value = self.expr(condition.clone())?;
                }

                Ok(())
            }
            Stmt::Block(stmts) => {
                // NOTE: decided to try this using only `Box`... probably cleaner to use `Rc` or
                //       `Gc`, but I want to see how long I can get away with this

                // remove previous environment data, replacing with a new one w/ empty values...
                let previous_environment_values = take(&mut self.environment.values);
                // ...and enclosed by the previous environment.
                let previous_environment_enclosing = self.environment.enclosing.take();
                self.environment.enclosing = Some(Box::new(Environment {
                    values: previous_environment_values,
                    enclosing: previous_environment_enclosing,
                }));

                // We have to catch the error, and not bubble it up immediately, so that we can
                // ensure we replace the previous environment below.
                let mut had_error = false;
                for stmt in stmts {
                    let result = self.stmt(stmt);
                    if result.is_err() {
                        had_error = true;
                    }
                }

                // remove temp. environment data and replace original
                let tmp_environment_enclosing = self.environment.enclosing.take();
                let previous_environment = *tmp_environment_enclosing.unwrap();
                self.environment = previous_environment;

                if had_error { anyhow::bail!("") } else { Ok(()) }
            }
            Stmt::Var(token, expr) => {
                let value = match expr {
                    Some(expr) => Some(self.expr(expr)?),
                    None => None,
                };

                self.environment.define(token.lexeme, value);

                Ok(())
            }
            Stmt::Expression(expr) => self.expr_stmt(expr),
            Stmt::Print(expr) => self.print_stmt(expr),
        }
    }

    fn expr(&mut self, expr: Expr) -> anyhow::Result<Value> {
        match expr {
            Expr::Assign(assign) => self.assign(assign),
            Expr::Binary(binary) => self.binary(binary),
            Expr::Grouping(grouping) => self.expr(*grouping.expression),
            Expr::Literal(literal) => self.literal(literal),
            Expr::Logical(logical) => self.logical(logical),
            Expr::Unary(unary) => self.unary(unary),
            Expr::Variable(token) => self.environment.get(&token),
        }
    }

    fn assign(&mut self, assign: Assign) -> anyhow::Result<Value> {
        let value = self.expr(*assign.value)?;
        self.environment.assign(&assign.name, value.clone())?;

        Ok(value)
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

    fn logical(&mut self, logical: Logical) -> anyhow::Result<Value> {
        let left = self.expr(*logical.left)?;

        match logical.operator.token_type {
            TokenType::Or => {
                if self.is_truthy(&left) {
                    return Ok(left);
                }
            }
            TokenType::And => {
                if !self.is_truthy(&left) {
                    return Ok(left);
                }
            }
            _ => unreachable!(),
        }

        self.expr(*logical.right)
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
            (TokenType::Bang, value) => Ok(Value::Bool(!self.is_truthy(&value))),
            _ => panic!(),
        }
    }

    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(val) => *val,
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

    fn expr_stmt(&mut self, expr: Expr) -> anyhow::Result<()> {
        self.expr(expr)?;
        Ok(())
    }

    fn print_stmt(&mut self, expr: Expr) -> anyhow::Result<()> {
        let value = self.expr(expr)?;
        println!("{value}");
        Ok(())
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
            interpreter.expr(parser.expr().unwrap()).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn test_interpret_expr_err() {
        let source = "(1 + 2) == ((2 * !false) + 5) / 3.0";
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).unwrap();
        let mut interpreter = Interpreter::new();

        assert!(interpreter.expr(parser.expr().unwrap()).is_err());
    }

    #[test]
    fn test_interpret_scope() {
        // run manually to check the print statement.

        let source = r#"
            var a = 0;
            var b = 1;

            {
              var b = 11;
              var c = 4;
              a = a + b;
            }

            a = a + b;

            print a;
        "#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).unwrap();
        let stmts = parser.parse().unwrap();
        assert_eq!(stmts.len(), 5);

        let mut interpreter = Interpreter::new();
        let res = interpreter.interpret(stmts);
        assert!(res.is_ok());
        assert!(interpreter.environment.enclosing.is_none());
        match interpreter.environment.values.get("a") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::Number(12.0))),
        }
        match interpreter.environment.values.get("b") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::Number(1.0))),
        }
        assert!(interpreter.environment.values.get("c").is_none());
    }

    #[test]
    fn test_interpret_logical_ops() {
        // run manually to check the print statement.

        let source = r#"
            var a = "hi" or 2;
            var b = nil or "yes";
            var c = nil and "maybe";
            var d = "possibly" and "maybe";
        "#;
        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).unwrap();
        let stmts = parser.parse().unwrap();
        assert_eq!(stmts.len(), 4);

        let mut interpreter = Interpreter::new();
        let res = interpreter.interpret(stmts);
        assert!(res.is_ok());
        assert!(interpreter.environment.enclosing.is_none());
        match interpreter.environment.values.get("a") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::String(String::from("hi")))),
        }
        match interpreter.environment.values.get("b") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::String(String::from("yes")))),
        }
        match interpreter.environment.values.get("c") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::Nil)),
        }
        match interpreter.environment.values.get("d") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::String(String::from("maybe")))),
        }
    }
}
