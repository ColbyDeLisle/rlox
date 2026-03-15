use super::{Interpreter, Signal, environment::Environment, value::Value};
use crate::stmt::Stmt;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

mod native_functions;
use crate::interpreter::class::Instance;
use crate::tokens::{Token, TokenType};
pub(super) use native_functions::Clock;

/// A trait representing a callable in Lox.
pub trait LoxCallable: Debug {
    /// Get the name of the callable.
    fn name(&self) -> String;
    /// Get the arity (i.e., number of arguments) of the callable.
    fn arity(&self) -> usize;
    /// Call the callable with the provided arguments.
    fn call(&self, interpreter: &mut Interpreter, args: &[Value]) -> anyhow::Result<Value>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoxFunction {
    pub(super) name: String,
    pub(super) params: Vec<String>,
    pub(super) body: Vec<Stmt>,
    pub(super) closure: Rc<RefCell<Environment>>,
    pub(super) is_initializer: bool,
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
            Ok(_) => {
                if self.is_initializer {
                    let this_token = Token::new(TokenType::This, "this".to_string(), None, 0);
                    Ok(Environment::get_at(self.closure.clone(), 0, &this_token)?)
                } else {
                    Ok(Value::Nil)
                }
            }
            Err(Signal::Return(val)) => {
                if self.is_initializer {
                    let this_token = Token::new(TokenType::This, "this".to_string(), None, 0);
                    Ok(Environment::get_at(self.closure.clone(), 0, &this_token)?)
                } else {
                    Ok(val)
                }
            }
            Err(Signal::RuntimeError(e)) => Err(e),
            Err(Signal::ResolveError) => anyhow::bail!(""),
        }
    }
}

impl LoxFunction {
    pub(crate) fn bind(&self, instance: Rc<RefCell<Instance>>) -> LoxFunction {
        let env = Rc::new(RefCell::new(Environment::new_with_enclosing(Some(
            self.closure.clone(),
        ))));
        env.borrow_mut()
            .define("this".to_string(), Some(Value::Instance(instance)));

        LoxFunction {
            name: self.name.clone(),
            params: self.params.clone(),
            body: self.body.clone(),
            closure: env,
            is_initializer: self.is_initializer,
        }
    }
}
