use super::{Interpreter, Signal, environment::Environment, value::Value};
use crate::stmt::Stmt;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

mod native_functions;
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

#[derive(Debug)]
pub(super) struct LoxFunction {
    pub(super) name: String,
    pub(super) params: Vec<String>,
    pub(super) body: Vec<Stmt>,
    pub(super) closure: Rc<RefCell<Environment>>,
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
            Err(Signal::ResolveError) => anyhow::bail!(""),
        }
    }
}
