use super::Interpreter;
use crate::interpreter::functions::LoxCallable;
use crate::interpreter::value::Value;
use crate::runtime_error;
use crate::tokens::Token;

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
