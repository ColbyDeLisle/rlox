use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use crate::interpreter::Interpreter;
use crate::interpreter::value::Value;
use crate::tokens::Token;
use super::functions::LoxCallable;

#[derive(Debug, Clone)]
pub(crate) struct Class {
    pub(crate) class_name: String,
}

impl Class {
    /// Create a new Lox class.
    pub(crate) fn new(class_name: String) -> Self {
        Self {
            class_name
        }
    }
}

impl Display for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.class_name)
    }
}

impl PartialEq for Class {
    fn eq(&self, other: &Self) -> bool {
        self.class_name == other.class_name
    }
}

impl Eq for Class {}

impl LoxCallable for Rc<Class> {
    fn name(&self) -> String {
        self.class_name.clone()
    }

    fn arity(&self) -> usize {
        0
    }

    fn call(&self, _interpreter: &mut Interpreter, _args: &[Value]) -> anyhow::Result<Value> {
        let instance = Instance::new(self.clone());

        Ok(Value::Instance(Rc::new(RefCell::new(instance))))
    }
}


#[derive(Debug, Clone)]
pub(crate) struct Instance {
    pub(crate) class: Rc<Class>,
    pub(crate) fields: HashMap<String, Value>
}

impl Instance {
    pub(crate) fn new(class: Rc<Class>) -> Self {
        Self {
            class,
            fields: HashMap::new(),
        }
    }

    pub(crate) fn get(&self, name: &Token) -> Option<Value> {
        self.fields.get(&name.lexeme).cloned()
    }
}

impl Display for Instance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} instance", self.class.class_name)
    }
}



