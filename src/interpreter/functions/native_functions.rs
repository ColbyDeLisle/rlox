use super::Interpreter;
use crate::interpreter::functions::LoxCallable;
use crate::interpreter::value::Value;

/// Lox's native `clock` function.
#[derive(Debug)]
pub(crate) struct Clock;

impl LoxCallable for Clock {
    fn name(&self) -> String {
        String::from("clock")
    }

    fn arity(&self) -> usize {
        0
    }

    fn call(&self, _interpreter: &mut Interpreter, _args: &[Value]) -> anyhow::Result<Value> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f32();

        Ok(Value::Number(time))
    }
}
