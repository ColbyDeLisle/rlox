use crate::{
    Literal,
    expr::{Assign, Binary, Call, Expr, Logical, Unary},
    resolver::Resolver,
    runtime_error,
    stmt::{Class, Function, If, Return, Stmt, Var, While},
    tokens::{Token, TokenType},
};
use std::collections::HashMap;
use std::{cell::RefCell, fmt::Debug, rc::Rc, result::Result};

pub(crate) mod environment;
use environment::Environment;

mod functions;
use functions::{Clock, Len, LoxCallable, LoxFunction, Pop, Push, Slice};

mod value;
use value::Value;

mod class;
use crate::expr::{Get, Index, IndexSet, Set, Super};
use class::{LoxClass, LoxInstance};

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
        env.borrow_mut().define(
            String::from("len"),
            Some(Value::NativeFunction(Rc::new(Len {}))),
        );
        env.borrow_mut().define(
            String::from("push"),
            Some(Value::NativeFunction(Rc::new(Push {}))),
        );
        env.borrow_mut().define(
            String::from("pop"),
            Some(Value::NativeFunction(Rc::new(Pop {}))),
        );
        env.borrow_mut().define(
            String::from("slice"),
            Some(Value::NativeFunction(Rc::new(Slice {}))),
        );

        Interpreter {
            globals: env.clone(),
            locals: HashMap::new(),
            environment: env,
            had_runtime_error: false,
        }
    }

    pub fn interpret(&mut self, stmts: Vec<Stmt>) -> InterpretResult {
        let mut resolver = Resolver::new(self);
        if resolver.resolve(&stmts).is_err() {
            return Err(Signal::ResolveError);
        }

        for stmt in &stmts {
            self.stmt(stmt)?;
        }

        match self.had_runtime_error {
            false => Ok(Value::Nil),
            true => Err(Signal::RuntimeError(anyhow::anyhow!(""))),
        }
    }

    fn stmt(&mut self, stmt: &Stmt) -> InterpretResult {
        match stmt {
            Stmt::If(If {
                condition,
                then_branch,
                else_branch,
            }) => {
                let condition_value = self.expr(condition).map_err(Signal::RuntimeError)?;

                if self.is_truthy(&condition_value) {
                    self.stmt(then_branch)?;
                } else if let Some(stmt) = else_branch {
                    self.stmt(stmt)?;
                }

                Ok(Value::Nil)
            }
            Stmt::While(While { condition, body }) => {
                let mut condition_value = self.expr(condition).map_err(Signal::RuntimeError)?;

                while self.is_truthy(&condition_value) {
                    self.stmt(body)?;
                    condition_value = self.expr(condition).map_err(Signal::RuntimeError)?;
                }

                Ok(Value::Nil)
            }
            Stmt::Block(stmts) => {
                let new_env = Environment::new_with_enclosing(Some(self.environment.clone()));
                self.execute_block_with_env(stmts, Rc::new(RefCell::new(new_env)))
            }
            Stmt::Class(Class {
                name,
                methods,
                superclass,
            }) => {
                let superclass = if let Some(class) = superclass {
                    let superclass_name = match class {
                        Expr::Variable(tok) => tok.clone(),
                        _ => unreachable!(),
                    };

                    let super_class = self.expr(class).map_err(Signal::RuntimeError)?;

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
                    let Stmt::Function(Function { name, params, body }) = method else {
                        unreachable!()
                    };
                    let m = LoxFunction {
                        name: name.lexeme.clone(),
                        params: params.iter().map(|t| t.lexeme.clone()).collect(),
                        body: body.clone(),
                        closure: self.environment.clone(),
                        is_initializer: name.lexeme == "init",
                    };

                    class_methods.insert(name.lexeme.clone(), m);
                }

                if superclass.is_some() {
                    let enc = self.environment.borrow_mut().enclosing.clone().unwrap();
                    self.environment = enc;
                }

                let class = Rc::new(LoxClass {
                    class_name: name.lexeme.clone(),
                    methods: class_methods,
                    superclass,
                });

                let result = self
                    .environment
                    .borrow_mut()
                    .assign(name, Value::Class(class));

                if result.is_err() {
                    self.had_runtime_error = true;
                    return Err(Signal::RuntimeError(anyhow::anyhow!("")));
                }

                Ok(Value::Nil)
            }
            Stmt::Var(Var { name, initializer }) => {
                let value = match initializer {
                    Some(expr) => Some(self.expr(expr).map_err(Signal::RuntimeError)?),
                    None => Some(Value::Nil),
                };

                self.environment.borrow_mut().define(name.lexeme.clone(), value);

                Ok(Value::Nil)
            }
            Stmt::Expression(expr) => self.expr_stmt(expr),
            Stmt::Print(expr) => self.print_stmt(expr),
            Stmt::Function(Function { name, params, body }) => {
                let f = LoxFunction {
                    name: name.lexeme.clone(),
                    params: params.iter().map(|t| t.lexeme.clone()).collect(),
                    body: body.clone(),
                    closure: self.environment.clone(),
                    is_initializer: false,
                };

                self.environment
                    .borrow_mut()
                    .define(f.name.clone(), Some(Value::Callable(Rc::new(f))));

                Ok(Value::Nil)
            }
            Stmt::Return(Return { value: expr, .. }) => match expr {
                None => Err(Signal::Return(Value::Nil)),
                Some(expr) => {
                    let val = self.expr(expr).map_err(Signal::RuntimeError)?;
                    Err(Signal::Return(val))
                }
            },
        }
    }

    fn execute_block_with_env(
        &mut self,
        stmts: &[Stmt],
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

    fn expr(&mut self, expr: &Expr) -> anyhow::Result<Value> {
        match expr {
            Expr::Array(elements) => self.array(elements),
            Expr::Assign(assign) => self.assign(assign),
            Expr::Binary(binary) => self.binary(binary),
            Expr::Call(call) => self.call(call),
            Expr::Get(get) => self.get(get),
            Expr::Index(index) => self.index(index),
            Expr::IndexSet(index_set) => self.index_set(index_set),
            Expr::Grouping(grouping) => self.expr(&grouping.expression),
            Expr::Literal(literal) => self.literal(literal),
            Expr::Logical(logical) => self.logical(logical),
            Expr::Set(set) => self.set(set),
            Expr::Super(supr) => self.supr(supr),
            Expr::This(this) => self.this(this),
            Expr::Unary(unary) => self.unary(unary),
            Expr::Variable(token) => self.lookup_var(token),
        }
    }

    fn assign(&mut self, assign: &Assign) -> anyhow::Result<Value> {
        let value = self.expr(&assign.value)?;

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

    fn literal(&self, literal: &Literal) -> anyhow::Result<Value> {
        let value = match literal {
            Literal::Number(val) => Value::Number(*val),
            Literal::String(val) => Value::String(val.clone()),
            Literal::Bool(val) => Value::Bool(*val),
            Literal::Nil => Value::Nil,
        };
        Ok(value)
    }

    fn logical(&mut self, logical: &Logical) -> anyhow::Result<Value> {
        let left = self.expr(&logical.left)?;

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

        self.expr(&logical.right)
    }

    fn unary(&mut self, unary: &Unary) -> anyhow::Result<Value> {
        let right_value = self.expr(&unary.right)?;

        match (&unary.operator.token_type, right_value) {
            (TokenType::Minus, Value::Number(val)) => Ok(Value::Number(-val)),
            (TokenType::Minus, _) => {
                let msg = "Operand must be a number.";
                runtime_error(Some(&unary.operator), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string())
            }
            (TokenType::Bang, value) => Ok(Value::Bool(!self.is_truthy(&value))),
            _ => unreachable!("unary operator must be Bang or Minus"),
        }
    }

    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(val) => *val,
            Value::Nil => false,
            _ => true,
        }
    }

    fn binary(&mut self, binary: &Binary) -> anyhow::Result<Value> {
        let left_value = self.expr(&binary.left)?;
        let right_value = self.expr(&binary.right)?;

        let value = match (&binary.operator.token_type, &left_value, &right_value) {
            (TokenType::Minus, Value::Number(left), Value::Number(right)) => {
                Value::Number(left - right)
            }
            (TokenType::Slash, Value::Number(left), Value::Number(right)) => match right {
                0.0 => {
                    let msg = "Division by zero.";
                    runtime_error(Some(&binary.operator), msg);
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
            (TokenType::BangEqual, Value::Array(x), Value::Array(y)) => {
                Value::Bool(!Rc::ptr_eq(x, y))
            }
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
            (TokenType::EqualEqual, Value::Array(x), Value::Array(y)) => {
                Value::Bool(Rc::ptr_eq(x, y))
            }
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
                runtime_error(Some(&binary.operator), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string());
            }
            (TokenType::Plus, _, _) => {
                let msg = "Operands must be two numbers or two strings.";
                runtime_error(Some(&binary.operator), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string())
            }
            _ => unreachable!("binary operator token type not handled"),
        };

        Ok(value)
    }

    fn expr_stmt(&mut self, expr: &Expr) -> InterpretResult {
        self.expr(expr).map_err(Signal::RuntimeError)
    }

    fn print_stmt(&mut self, expr: &Expr) -> InterpretResult {
        let val = self.expr(expr).map_err(Signal::RuntimeError)?;
        println!("{val}");
        Ok(Value::Nil)
    }

    fn call(&mut self, call: &Call) -> anyhow::Result<Value> {
        let callee = self.expr(&call.callee)?;
        let mut args: Vec<Value> = vec![];

        for arg in &call.args {
            args.push(self.expr(arg)?);
        }

        match callee {
            Value::Callable(f) => f.call(self, &args, &call.paren),
            Value::Class(f) => f.call(self, &args, &call.paren),
            Value::NativeFunction(f) => f.call(self, &args, &call.paren),
            _ => {
                let msg = "Can only call functions and classes.";
                runtime_error(Some(&call.paren), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string())
            }
        }
    }

    fn get(&mut self, get: &Get) -> anyhow::Result<Value> {
        let object = self.expr(&get.expr)?;

        match object {
            Value::Instance(i) => {
                let value = LoxInstance::get(i.clone(), &get.name);
                match value {
                    Some(value) => Ok(value),
                    None => {
                        let msg = format!("Undefined property '{}'.", get.name.lexeme);
                        runtime_error(Some(&get.name), &msg);
                        self.had_runtime_error = true;
                        anyhow::bail!(msg)
                    }
                }
            }
            _ => {
                let msg = "Only instances have properties.";
                runtime_error(Some(&get.name), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string());
            }
        }
    }

    fn set(&mut self, set: &Set) -> anyhow::Result<Value> {
        let object = self.expr(&set.expr)?;

        match object {
            Value::Instance(i) => {
                let value = self.expr(&set.value)?;
                i.borrow_mut().set(&set.name, value.clone());
                Ok(value)
            }
            _ => {
                let msg = "Only instances have fields.";
                runtime_error(Some(&set.name), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string());
            }
        }
    }

    fn array(&mut self, elements: &[Expr]) -> anyhow::Result<Value> {
        let mut values = Vec::with_capacity(elements.len());
        for element in elements {
            values.push(self.expr(element)?);
        }
        Ok(Value::Array(Rc::new(RefCell::new(values))))
    }

    fn index(&mut self, index: &Index) -> anyhow::Result<Value> {
        let object = self.expr(&index.object)?;
        let idx = self.expr(&index.index)?;

        match object {
            Value::Array(arr) => {
                let len = arr.borrow().len();
                let i = self.array_index(&idx, len, &index.bracket)?;
                Ok(arr.borrow()[i].clone())
            }
            _ => {
                let msg = "Can only index into arrays.";
                runtime_error(Some(&index.bracket), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string());
            }
        }
    }

    fn index_set(&mut self, index_set: &IndexSet) -> anyhow::Result<Value> {
        let object = self.expr(&index_set.object)?;
        let idx = self.expr(&index_set.index)?;
        let value = self.expr(&index_set.value)?;

        match object {
            Value::Array(arr) => {
                let len = arr.borrow().len();
                let i = self.array_index(&idx, len, &index_set.bracket)?;
                arr.borrow_mut()[i] = value.clone();
                Ok(value)
            }
            _ => {
                let msg = "Can only index into arrays.";
                runtime_error(Some(&index_set.bracket), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string());
            }
        }
    }

    /// Validate a Lox value used as an array index against `len`, returning the `usize` index.
    fn array_index(&mut self, idx: &Value, len: usize, bracket: &Token) -> anyhow::Result<usize> {
        let n = match idx {
            Value::Number(n) => *n,
            _ => {
                let msg = "Array index must be a number.";
                runtime_error(Some(bracket), msg);
                self.had_runtime_error = true;
                anyhow::bail!(msg.to_string());
            }
        };

        if n < 0.0 || n.fract() != 0.0 {
            let msg = "Array index must be a non-negative integer.";
            runtime_error(Some(bracket), msg);
            self.had_runtime_error = true;
            anyhow::bail!(msg.to_string());
        }

        let i = n as usize;
        if i >= len {
            let msg = "Array index out of bounds.";
            runtime_error(Some(bracket), msg);
            self.had_runtime_error = true;
            anyhow::bail!(msg.to_string());
        }

        Ok(i)
    }

    fn this(&mut self, this: &Token) -> anyhow::Result<Value> {
        self.lookup_var(this)
    }

    fn supr(&mut self, supr: &Super) -> anyhow::Result<Value> {
        // The resolver guarantees that every `super` expression is resolved.
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

        let method = superclass.find_method(supr.method.lexeme.as_str());

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
