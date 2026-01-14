use crate::{
    Literal, error,
    expr::{Binary, Expr, Unary},
    stmt::Stmt,
    tokens::TokenType,
};
use std::{
    cell::RefCell,
    fmt::{Debug, Display, Formatter},
    rc::Rc,
    result::Result,
};

mod environment;
use environment::Environment;
mod native_functions;
use native_functions::Clock;

use crate::expr::{Assign, Call, Logical};

#[derive(Debug, Clone)]
pub enum Value {
    Number(f32),
    String(String),
    Bool(bool),
    Callable(Rc<dyn LoxCallable>),
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
            Value::Number(val) => write!(f, "{}", val),
            Value::String(val) => write!(f, "{}", val),
            Value::Bool(val) => write!(f, "{}", val),
            Value::Callable(val) => write!(f, "<fun {}>", val.name()),
            Value::Nil => write!(f, "Nil"),
        }
    }
}

pub enum Signal {
    Return(Value),
    RuntimeError(anyhow::Error),
}

pub type InterpretResult = Result<Value, Signal>;

pub(crate) trait LoxCallable: Debug {
    fn name(&self) -> String;
    fn arity(&self) -> usize;
    fn call(&self, interpreter: &mut Interpreter, args: &[Value]) -> anyhow::Result<Value>;
}

#[derive(Debug)]
pub(crate) struct LoxFunction {
    name: String,
    params: Vec<String>,
    body: Vec<Stmt>,
    closure: Rc<RefCell<Environment>>,
}

impl LoxCallable for LoxFunction {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn arity(&self) -> usize {
        self.params.len()
    }

    fn call(&self, interpreter: &mut Interpreter, args: &[Value]) -> anyhow::Result<Value> {
        let env = Rc::new(RefCell::new(Environment::new_with_enclosing(Some(
            self.closure.clone(),
        ))));

        for i in 0..self.params.len() {
            env.borrow_mut()
                .define(self.params[i].clone(), Some(args[i].clone()));
        }

        let body_result = interpreter.execute_block_with_env(self.body.clone(), env);
        match body_result {
            // n.b. we return Nil from a successful function call w/o an explicit `return`
            Ok(_) => Ok(Value::Nil),
            Err(Signal::Return(val)) => Ok(val),
            Err(Signal::RuntimeError(e)) => Err(e),
        }
    }
}

#[derive(Debug, Default)]
pub struct Interpreter {
    globals: Rc<RefCell<Environment>>,
    environment: Rc<RefCell<Environment>>,
    had_runtime_error: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        let env = Rc::new(RefCell::new(Environment::default()));
        env.borrow_mut().define(
            String::from("clock"),
            Some(Value::Callable(Rc::new(Clock {}))),
        );

        Interpreter {
            globals: env.clone(),
            environment: env,
            had_runtime_error: false,
        }
    }

    pub fn interpret(&mut self, stmts: Vec<Stmt>) -> InterpretResult {
        for stmt in stmts {
            self.stmt(stmt)?;
        }

        match self.had_runtime_error {
            false => Ok(Value::Nil),
            true => Err(Signal::RuntimeError(anyhow::anyhow!(""))),
        }
    }

    fn stmt(&mut self, stmt: Stmt) -> InterpretResult {
        match stmt {
            Stmt::If(condition, then_branch, else_branch) => {
                let condition_value = match self.expr(condition) {
                    Ok(val) => val,
                    Err(e) => {
                        return Err(Signal::RuntimeError(e));
                    }
                };

                if self.is_truthy(&condition_value) {
                    self.stmt(*then_branch)?;
                } else if let Some(stmt) = else_branch {
                    self.stmt(*stmt)?;
                }

                Ok(Value::Nil)
            }
            Stmt::While(condition, body) => {
                let mut condition_value = match self.expr(condition.clone()) {
                    Ok(val) => val,
                    Err(e) => {
                        return Err(Signal::RuntimeError(e));
                    }
                };

                while self.is_truthy(&condition_value) {
                    self.stmt(*body.clone())?;
                    condition_value = match self.expr(condition.clone()) {
                        Ok(val) => val,
                        Err(e) => {
                            return Err(Signal::RuntimeError(e));
                        }
                    };
                }

                Ok(Value::Nil)
            }
            Stmt::Block(stmts) => {
                let new_env = Environment::new_with_enclosing(Some(self.environment.clone()));
                self.execute_block_with_env(stmts, Rc::new(RefCell::new(new_env)))
            }
            Stmt::Var(token, expr) => {
                let value = match expr {
                    Some(expr) => {
                        let value = match self.expr(expr) {
                            Ok(val) => val,
                            Err(e) => {
                                return Err(Signal::RuntimeError(e));
                            }
                        };
                        Some(value)
                    }
                    None => None,
                };

                self.environment.borrow_mut().define(token.lexeme, value);

                Ok(Value::Nil)
            }
            Stmt::Expression(expr) => self.expr_stmt(expr),
            Stmt::Print(expr) => self.print_stmt(expr),
            Stmt::Function(name, params, body) => {
                let f = LoxFunction {
                    name: name.lexeme,
                    params: params.iter().map(|t| t.lexeme.clone()).collect(),
                    body,
                    closure: self.environment.clone(),
                };

                self.environment
                    .borrow_mut()
                    .define(f.name.clone(), Some(Value::Callable(Rc::new(f))));

                Ok(Value::Nil)
            }
            Stmt::Return(_, expr) => match expr {
                None => Err(Signal::Return(Value::Nil)),
                Some(expr_) => {
                    let value = self.expr(expr_);
                    match value {
                        Ok(val) => Err(Signal::Return(val)),
                        Err(e) => Err(Signal::RuntimeError(e)),
                    }
                }
            },
        }
    }

    pub(crate) fn execute_block_with_env(
        &mut self,
        stmts: Vec<Stmt>,
        env: Rc<RefCell<Environment>>,
    ) -> InterpretResult {
        let old_env = self.environment.clone();
        self.environment = env;

        // We have to catch the error, and not bubble it up immediately, so that we can
        // ensure we replace the previous environment below.
        let mut had_error = false;

        for stmt in stmts {
            match self.stmt(stmt) {
                Ok(_) => (),
                Err(Signal::Return(val)) => {
                    self.environment = old_env;
                    return if had_error {
                        Err(Signal::RuntimeError(anyhow::anyhow!("")))
                    } else {
                        Err(Signal::Return(val))
                    };
                }
                Err(Signal::RuntimeError(_)) => {
                    had_error = true;
                }
            }
        }

        self.environment = old_env;

        if had_error {
            Err(Signal::RuntimeError(anyhow::anyhow!("")))
        } else {
            Ok(Value::Nil)
        }
    }

    fn expr(&mut self, expr: Expr) -> anyhow::Result<Value> {
        match expr {
            Expr::Assign(assign) => self.assign(assign),
            Expr::Binary(binary) => self.binary(binary),
            Expr::Call(call) => self.call(call),
            Expr::Grouping(grouping) => self.expr(*grouping.expression),
            Expr::Literal(literal) => self.literal(literal),
            Expr::Logical(logical) => self.logical(logical),
            Expr::Unary(unary) => self.unary(unary),
            Expr::Variable(token) => self.environment.borrow().get(&token),
        }
    }

    fn assign(&mut self, assign: Assign) -> anyhow::Result<Value> {
        let value = self.expr(*assign.value)?;
        self.environment
            .borrow_mut()
            .assign(&assign.name, value.clone())?;

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

    fn expr_stmt(&mut self, expr: Expr) -> InterpretResult {
        let value = self.expr(expr);
        match value {
            Ok(val) => Ok(val),
            Err(e) => Err(Signal::RuntimeError(e)),
        }
    }

    fn print_stmt(&mut self, expr: Expr) -> InterpretResult {
        let value = self.expr(expr);
        match value {
            Ok(val) => {
                println!("{val}");
                Ok(Value::Nil)
            }
            Err(e) => Err(Signal::RuntimeError(e)),
        }
    }

    fn call(&mut self, call: Call) -> anyhow::Result<Value> {
        let callee = self.expr(*call.callee)?;
        let mut args: Vec<Value> = vec![];

        for arg in call.args {
            args.push(self.expr(arg)?);
        }

        if let Value::Callable(f) = callee {
            if args.len() != f.arity() {
                let msg = format!("Expected {} arguments; got {}.", f.arity(), args.len(),);
                error(Some(&call.paren.clone()), &msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg)
            }

            f.call(self, &args)
        } else {
            let msg = "Can only call functions or classes.";
            error(Some(&call.paren.clone()), msg);
            self.had_runtime_error = true;
            anyhow::bail!(msg.to_string())
        }
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
        assert!(interpreter.environment.borrow().enclosing.is_none());
        match interpreter.environment.borrow().values.get("a") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::Number(12.0))),
        }
        match interpreter.environment.borrow().values.get("b") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::Number(1.0))),
        }
        assert!(interpreter.environment.borrow().values.get("c").is_none());
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
        assert!(interpreter.environment.borrow().enclosing.is_none());
        match interpreter.environment.borrow().values.get("a") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::String(String::from("hi")))),
        }
        match interpreter.environment.borrow().values.get("b") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::String(String::from("yes")))),
        }
        match interpreter.environment.borrow().values.get("c") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::Nil)),
        }
        match interpreter.environment.borrow().values.get("d") {
            None => assert!(false),
            Some(value) => assert_eq!(*value, Some(Value::String(String::from("maybe")))),
        }
    }
}
