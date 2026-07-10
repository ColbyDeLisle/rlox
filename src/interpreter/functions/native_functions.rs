use super::Interpreter;
use crate::interpreter::functions::LoxCallable;
use crate::interpreter::value::Value;
use crate::runtime_error;
use crate::tokens::Token;
use std::cell::RefCell;
use std::rc::Rc;

/// Lox's native `clock` function.
#[derive(Debug)]
pub(crate) struct Clock;

impl LoxCallable for Clock {
    fn name(&self) -> String {
        String::new()
    }

    fn arity(&self) -> usize {
        0
    }

    fn call(
        &self,
        _interpreter: &mut Interpreter,
        args: &[Value],
        paren_token: &Token,
    ) -> anyhow::Result<Value> {
        if args.len() != self.arity() {
            let msg = format!(
                "Expected {} arguments but got {}.",
                self.arity(),
                args.len(),
            );
            runtime_error(Some(paren_token), &msg);
            anyhow::bail!(msg)
        }

        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f32();

        Ok(Value::Number(time))
    }
}

/// Lox's native `len` function: the number of elements in an array.
#[derive(Debug)]
pub(crate) struct Len;

impl LoxCallable for Len {
    fn name(&self) -> String {
        String::from("len")
    }

    fn arity(&self) -> usize {
        1
    }

    fn call(
        &self,
        _interpreter: &mut Interpreter,
        args: &[Value],
        paren_token: &Token,
    ) -> anyhow::Result<Value> {
        check_arity(self, args, paren_token)?;

        match &args[0] {
            Value::Array(arr) => Ok(Value::Number(arr.borrow().len() as f32)),
            _ => {
                let msg = "Argument to 'len' must be an array.";
                runtime_error(Some(paren_token), msg);
                anyhow::bail!(msg)
            }
        }
    }
}

/// Lox's native `push` function: append a value to the end of an array.
#[derive(Debug)]
pub(crate) struct Push;

impl LoxCallable for Push {
    fn name(&self) -> String {
        String::from("push")
    }

    fn arity(&self) -> usize {
        2
    }

    fn call(
        &self,
        _interpreter: &mut Interpreter,
        args: &[Value],
        paren_token: &Token,
    ) -> anyhow::Result<Value> {
        check_arity(self, args, paren_token)?;

        match &args[0] {
            Value::Array(arr) => {
                arr.borrow_mut().push(args[1].clone());
                Ok(Value::Nil)
            }
            _ => {
                let msg = "First argument to 'push' must be an array.";
                runtime_error(Some(paren_token), msg);
                anyhow::bail!(msg)
            }
        }
    }
}

/// Lox's native `pop` function: remove and return the last element of an array.
#[derive(Debug)]
pub(crate) struct Pop;

impl LoxCallable for Pop {
    fn name(&self) -> String {
        String::from("pop")
    }

    fn arity(&self) -> usize {
        1
    }

    fn call(
        &self,
        _interpreter: &mut Interpreter,
        args: &[Value],
        paren_token: &Token,
    ) -> anyhow::Result<Value> {
        check_arity(self, args, paren_token)?;

        match &args[0] {
            Value::Array(arr) => {
                let popped = arr.borrow_mut().pop();
                match popped {
                    Some(value) => Ok(value),
                    None => {
                        let msg = "Cannot pop from an empty array.";
                        runtime_error(Some(paren_token), msg);
                        anyhow::bail!(msg)
                    }
                }
            }
            _ => {
                let msg = "Argument to 'pop' must be an array.";
                runtime_error(Some(paren_token), msg);
                anyhow::bail!(msg)
            }
        }
    }
}

/// Emit a Lox arity-mismatch runtime error if `args` doesn't match the callable's arity.
fn check_arity(
    callable: &dyn LoxCallable,
    args: &[Value],
    paren_token: &Token,
) -> anyhow::Result<()> {
    if args.len() != callable.arity() {
        let msg = format!(
            "Expected {} arguments but got {}.",
            callable.arity(),
            args.len(),
        );
        runtime_error(Some(paren_token), &msg);
        anyhow::bail!(msg)
    }

    Ok(())
}

/// Lox's native `slice` function: a copied sub-array `slice(a, start, end)`, end-exclusive.
#[derive(Debug)]
pub(crate) struct Slice;

impl LoxCallable for Slice {
    fn name(&self) -> String {
        String::from("slice")
    }

    fn arity(&self) -> usize {
        3
    }

    fn call(
        &self,
        _interpreter: &mut Interpreter,
        args: &[Value],
        paren_token: &Token,
    ) -> anyhow::Result<Value> {
        check_arity(self, args, paren_token)?;

        let arr = match &args[0] {
            Value::Array(arr) => arr,
            _ => {
                let msg = "First argument to 'slice' must be an array.";
                runtime_error(Some(paren_token), msg);
                anyhow::bail!(msg)
            }
        };

        let len = arr.borrow().len();
        let start = slice_bound(&args[1], paren_token)?;
        let end = slice_bound(&args[2], paren_token)?;

        if start > end || end > len {
            let msg = "Slice bounds out of range.";
            runtime_error(Some(paren_token), msg);
            anyhow::bail!(msg)
        }

        // Copy semantics: the slice is a fresh array independent of the source.
        let sliced = arr.borrow()[start..end].to_vec();
        Ok(Value::Array(Rc::new(RefCell::new(sliced))))
    }
}

/// Validate a Lox value used as a slice bound, returning it as a `usize`.
fn slice_bound(value: &Value, paren_token: &Token) -> anyhow::Result<usize> {
    let n = match value {
        Value::Number(n) => *n,
        _ => {
            let msg = "Slice bounds must be numbers.";
            runtime_error(Some(paren_token), msg);
            anyhow::bail!(msg)
        }
    };

    if n < 0.0 || n.fract() != 0.0 {
        let msg = "Slice bounds must be non-negative integers.";
        runtime_error(Some(paren_token), msg);
        anyhow::bail!(msg)
    }

    Ok(n as usize)
}
