use crate::{
    Literal,
    expr::{Assign, Binary, Call, Expr, Logical, Unary},
    resolver::Resolver,
    runtime_error,
    stmt::Stmt,
    tokens::{Token, TokenType},
};
use std::collections::HashMap;
use std::{cell::RefCell, fmt::Debug, rc::Rc, result::Result};

pub(crate) mod environment;
use environment::Environment;

mod functions;
use functions::{Clock, LoxCallable, LoxFunction};

mod value;
use value::Value;

mod class;
use crate::expr::{Get, Set, Super};
use class::{Class, Instance};

#[derive(Debug)]
pub enum Signal {
    Return(Value),
    ResolveError,
    RuntimeError(anyhow::Error),
}

pub type InterpretResult = Result<Value, Signal>;

#[derive(Debug, Default)]
pub struct Interpreter {
    globals: Rc<RefCell<Environment>>,
    locals: HashMap<Token, usize>,
    environment: Rc<RefCell<Environment>>,
    had_runtime_error: bool,
}

impl Interpreter {
    pub fn new() -> Self {
        let env = Rc::new(RefCell::new(Environment::default()));
        env.borrow_mut().define(
            String::from("clock"),
            Some(Value::NativeFunction(Rc::new(Clock {}))),
        );

        Interpreter {
            globals: env.clone(),
            locals: HashMap::new(),
            environment: env,
            had_runtime_error: false,
        }
    }

    pub fn interpret(&mut self, stmts: Vec<Stmt>) -> InterpretResult {
        let mut resolver = Resolver::new(std::mem::take(self));
        if resolver.resolve(&stmts).is_err() {
            return InterpretResult::Err(Signal::ResolveError);
        }
        *self = std::mem::take(&mut resolver.interpreter);

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
            Stmt::Class(name, methods, super_class) => {
                let superclass = if let Some(class) = super_class {
                    let superclass_name = match &class {
                        Expr::Variable(tok) => tok.clone(),
                        _ => unreachable!(),
                    };

                    let super_class = match self.expr(class) {
                        Ok(val) => val,
                        Err(e) => {
                            return Err(Signal::RuntimeError(e));
                        }
                    };

                    match super_class {
                        Value::Class(c) => Some(c),
                        _ => {
                            let msg = "Superclass must be a class.";
                            runtime_error(Some(&superclass_name), msg);
                            self.had_runtime_error = true;
                            return Err(Signal::RuntimeError(anyhow::anyhow!(msg)));
                        }
                    }
                } else {
                    None
                };

                self.environment
                    .borrow_mut()
                    .define(name.lexeme.clone(), None);

                if let Some(cls) = superclass.clone() {
                    let new_env = Environment::new_with_enclosing(Some(self.environment.clone()));
                    self.environment = Rc::new(RefCell::new(new_env));
                    self.environment
                        .borrow_mut()
                        .define("super".to_string(), Some(Value::Class(cls.clone())));
                }

                let mut class_methods = HashMap::new();
                for method in methods {
                    let Stmt::Function(name, params, body) = method else {
                        unreachable!()
                    };
                    let m = LoxFunction {
                        name: name.lexeme.clone(),
                        params: params.iter().map(|t| t.lexeme.clone()).collect(),
                        body,
                        closure: self.environment.clone(),
                        is_initializer: name.lexeme == "init",
                    };

                    class_methods.insert(name.lexeme.clone(), m);
                }

                if superclass.is_some() {
                    let enc = self.environment.borrow_mut().enclosing.clone().unwrap();
                    self.environment = enc;
                }

                let class = Rc::new(Class {
                    class_name: name.lexeme.clone(),
                    methods: class_methods,
                    superclass,
                });

                self.environment
                    .borrow_mut()
                    .assign(&name, Value::Class(class));

                Ok(Value::Nil)
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
                    None => Some(Value::Nil),
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
                    is_initializer: false,
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

    fn execute_block_with_env(
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
                Err(Signal::ResolveError) | Err(Signal::RuntimeError(_)) => {
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
            Expr::Get(get) => self.get(get),
            Expr::Grouping(grouping) => self.expr(*grouping.expression),
            Expr::Literal(literal) => self.literal(literal),
            Expr::Logical(logical) => self.logical(logical),
            Expr::Set(set) => self.set(set),
            Expr::Super(supr) => self.supr(supr),
            Expr::This(this) => self.this(this),
            Expr::Unary(unary) => self.unary(unary),
            Expr::Variable(token) => self.lookup_var(&token),
        }
    }

    fn assign(&mut self, assign: Assign) -> anyhow::Result<Value> {
        let value = self.expr(*assign.value)?;

        if let Some(distance) = self.locals.get(&assign.name) {
            Environment::assign_at(
                self.environment.clone(),
                *distance,
                &assign.name,
                value.clone(),
            )?;
        } else {
            self.globals
                .borrow_mut()
                .assign(&assign.name, value.clone())?;
        }

        Ok(value)
    }

    fn literal(&self, literal: Literal) -> anyhow::Result<Value> {
        let value = match literal {
            Literal::Number(val) => Value::Number(val),
            Literal::String(val) => Value::String(val),
            Literal::Bool(val) => Value::Bool(val),
            Literal::Nil => Value::Nil,
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
                runtime_error(Some(&unary.operator.clone()), msg);
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
            (TokenType::Slash, Value::Number(left), Value::Number(right)) => match right {
                0.0 => {
                    let msg = "Division by zero.";
                    runtime_error(Some(&binary.operator.clone()), msg);
                    self.had_runtime_error = true;
                    anyhow::bail!(msg.to_string());
                }
                _ => Value::Number(left / right),
            },
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
            (TokenType::BangEqual, Value::Bool(left), Value::Bool(right)) => {
                Value::Bool(*left != *right)
            }
            (TokenType::BangEqual, Value::String(left), Value::String(right)) => {
                Value::Bool(*left != *right)
            }
            (TokenType::BangEqual, Value::Nil, Value::Nil) => Value::Bool(false),
            (TokenType::BangEqual, Value::Callable(f), Value::Callable(g)) => Value::Bool(f != g),
            (TokenType::BangEqual, Value::Class(x), Value::Class(y)) => Value::Bool(x != y),
            (TokenType::BangEqual, Value::Instance(x), Value::Instance(y)) => Value::Bool(x != y),
            (TokenType::BangEqual, _, _) => Value::Bool(true),
            (TokenType::EqualEqual, Value::Number(left), Value::Number(right)) => {
                Value::Bool(*left == *right)
            }
            (TokenType::EqualEqual, Value::Bool(left), Value::Bool(right)) => {
                Value::Bool(*left == *right)
            }
            (TokenType::EqualEqual, Value::String(left), Value::String(right)) => {
                Value::Bool(*left == *right)
            }
            (TokenType::EqualEqual, Value::Nil, Value::Nil) => Value::Bool(true),
            (TokenType::EqualEqual, Value::Callable(f), Value::Callable(g)) => Value::Bool(f == g),
            (TokenType::EqualEqual, Value::Class(x), Value::Class(y)) => Value::Bool(x == y),
            (TokenType::EqualEqual, Value::Instance(x), Value::Instance(y)) => Value::Bool(x == y),
            (TokenType::EqualEqual, _, _) => Value::Bool(false),
            (
                TokenType::Minus
                | TokenType::Slash
                | TokenType::Star
                | TokenType::Greater
                | TokenType::GreaterEqual
                | TokenType::Less
                | TokenType::LessEqual,
                _,
                _,
            ) => {
                let msg = "Operands must be numbers.";
                runtime_error(Some(&binary.operator.clone()), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string());
            }
            (TokenType::Plus, _, _) => {
                let msg = "Operands must be two numbers or two strings.";
                runtime_error(Some(&binary.operator.clone()), msg);
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

        // TODO: fix duplication here?
        if let Value::Callable(f) = callee {
            if args.len() != f.arity() {
                let msg = format!("Expected {} arguments but got {}.", f.arity(), args.len(),);
                runtime_error(Some(&call.paren.clone()), &msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg)
            }

            f.call(self, &args)
        } else if let Value::Class(f) = callee {
            if args.len() != f.arity() {
                let msg = format!("Expected {} arguments but got {}.", f.arity(), args.len(),);
                runtime_error(Some(&call.paren.clone()), &msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg)
            }

            f.call(self, &args)
        } else {
            let msg = "Can only call functions and classes.";
            runtime_error(Some(&call.paren.clone()), msg);
            self.had_runtime_error = true;
            anyhow::bail!(msg.to_string())
        }
    }

    fn get(&mut self, get: Get) -> anyhow::Result<Value> {
        let name = get.name.clone();
        let object = self.expr(*get.expr)?;

        match object {
            Value::Instance(i) => {
                let value = Instance::get(i.clone(), &name);
                match value {
                    Some(value) => Ok(value),
                    None => {
                        let msg = format!("Undefined property '{}'.", name.lexeme);
                        runtime_error(Some(&name.clone()), &msg);
                        self.had_runtime_error = true;
                        anyhow::bail!(msg)
                    }
                }
            }
            _ => {
                let msg = "Only instances have properties.";
                runtime_error(Some(&name.clone()), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string());
            }
        }
    }

    fn set(&mut self, set: Set) -> anyhow::Result<Value> {
        let name = set.name.clone();
        let object = self.expr(*set.expr)?;

        match object {
            Value::Instance(i) => {
                let value = self.expr(*set.value)?;
                i.borrow_mut().set(&name, value.clone());
                Ok(value)
            }
            _ => {
                let msg = "Only instances have fields.";
                runtime_error(Some(&name), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string());
            }
        }
    }

    fn this(&mut self, this: Token) -> anyhow::Result<Value> {
        self.lookup_var(&this)
    }

    fn supr(&mut self, supr: Super) -> anyhow::Result<Value> {
        let distance = self.locals.get(&supr.keyword).unwrap();

        let superclass =
            match Environment::get_at(self.environment.clone(), *distance, &supr.keyword)? {
                Value::Class(c) => c,
                _ => unreachable!(),
            };

        let this_token = Token::new(TokenType::This, "this".to_string(), None, 0);
        let object = Environment::get_at(self.environment.clone(), *distance - 1, &this_token)?;
        let object = match object {
            Value::Instance(i) => i,
            _ => unreachable!(),
        };

        let method = superclass.methods.get(supr.method.lexeme.as_str());

        if let Some(method) = method {
            Ok(Value::Callable(Rc::new(method.bind(object))))
        } else {
            let msg = format!("Undefined property '{}'.", supr.method.lexeme);
            runtime_error(Some(&supr.method), msg.as_str());
            self.had_runtime_error = true;
            anyhow::bail!(msg);
        }
    }

    pub(crate) fn resolve(&mut self, name: Token, depth: usize) {
        self.locals.insert(name, depth);
    }

    fn lookup_var(&self, name: &Token) -> anyhow::Result<Value> {
        if let Some(distance) = self.locals.get(name) {
            Environment::get_at(self.environment.clone(), *distance, name)
        } else {
            self.globals.borrow().get(name)
        }
    }
}
