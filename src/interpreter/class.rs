use super::functions::{LoxCallable, LoxFunction};
use crate::interpreter::Interpreter;
use crate::interpreter::value::Value;
use crate::tokens::Token;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::runtime_error;

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug, Clone)]
pub(super) struct Class {
    pub(super) class_name: String,
    pub(super) methods: HashMap<String, LoxFunction>,
    pub(super) superclass: Option<Rc<Self>>,
}

impl Class {
    pub(super) fn find_method(&self, name: &str) -> Option<LoxFunction> {
        if let Some(method) = self.methods.get(name) {
            return Some(method.clone());
        } else if let Some(superclass) = &self.superclass {
            return superclass.find_method(name);
        }

        None
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
        if let Some(initializer) = self.methods.get("init") {
            initializer.arity()
        } else {
            0
        }
    }

    fn call(
        &self,
        interpreter: &mut Interpreter,
        args: &[Value],
        paren_token: &Token,
    ) -> anyhow::Result<Value> {
        let instance = Rc::new(RefCell::new(Instance::new(self.clone())));

        if let Some(initializer) = self.find_method("init") {
            let init = LoxFunction::bind(&initializer, instance.clone());
            init.call(interpreter, args, paren_token)?;
        } else {
            // still need to check arity, to "call" the default initializer
            if args.len() != self.arity() {
                let msg = format!(
                    "Expected {} arguments but got {}.",
                    self.arity(),
                    args.len(),
                );
                runtime_error(Some(paren_token), &msg);
                anyhow::bail!(msg)
            }
        }

        Ok(Value::Instance(instance))
    }
}

#[derive(Debug, Clone)]
pub(super) struct Instance {
    pub(super) class: Rc<Class>,
    pub(super) fields: HashMap<String, Value>,
    id: usize,
}

impl Instance {
    pub(super) fn new(class: Rc<Class>) -> Self {
        Self {
            class,
            fields: HashMap::new(),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub(super) fn get(instance: Rc<RefCell<Instance>>, name: &Token) -> Option<Value> {
        let self_ = instance.borrow();

        if let Some(field) = self_.fields.get(&name.lexeme) {
            return Some(field.clone());
        }

        if let Some(method) = self_.class.find_method(&name.lexeme) {
            return Some(Value::Callable(Rc::new(method.bind(instance.clone()))));
        }

        if let Some(superclass) = self_.class.superclass.clone() {
            if let Some(method) = superclass.find_method(&name.lexeme) {
                return Some(Value::Callable(Rc::new(method.bind(instance.clone()))));
            }
        }

        None
    }

    pub(super) fn set(&mut self, name: &Token, value: Value) {
        self.fields.insert(name.lexeme.to_owned(), value);
    }
}

impl Display for Instance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} instance", self.class.class_name)
    }
}

impl PartialEq for Instance {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Instance {}
