use crate::{Token, error, interpreter::Value};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Debug, Default, Clone)]
pub(crate) struct Environment {
    pub(crate) enclosing: Option<Rc<RefCell<Environment>>>,
    pub(crate) values: HashMap<String, Option<Value>>,
}

impl Environment {
    pub(crate) fn new_with_enclosing(enclosing: Option<Rc<RefCell<Environment>>>) -> Self {
        Environment {
            enclosing,
            values: HashMap::new(),
        }
    }

    pub(crate) fn define(&mut self, name: String, value: Option<Value>) {
        self.values.insert(name, value);
    }

    pub(crate) fn get(&self, name: &Token) -> anyhow::Result<Value> {
        let value = self.values.get(&name.lexeme);

        match value {
            Some(Some(val)) => Ok(val.clone()),
            _ => match &self.enclosing {
                Some(environment) => environment.borrow().get(name),
                None => {
                    let msg = format!("Undefined variable '{}'.", &name.lexeme);
                    error(Some(name), msg.as_str());
                    anyhow::bail!(msg)
                }
            },
        }
    }

    pub(crate) fn get_at(
        environment: Rc<RefCell<Environment>>,
        distance: usize,
        name: &Token,
    ) -> anyhow::Result<Value> {
        let env = Self::ancestor(environment, distance);

        if let Some(env) = env {
            env.borrow().get(name)
        } else {
            let msg = format!(
                "Undefined variable '{}' at distance {distance}.",
                &name.lexeme
            );
            error(Some(name), msg.as_str());
            anyhow::bail!(msg)
        }
    }

    pub(crate) fn assign_at(
        environment: Rc<RefCell<Environment>>,
        distance: usize,
        name: &Token,
        value: Value,
    ) -> anyhow::Result<()> {
        let env = Self::ancestor(environment, distance);

        if let Some(env) = env {
            env.borrow_mut().assign(name, value)
        } else {
            let msg = format!(
                "Undefined variable '{}' at distance {distance}.",
                &name.lexeme
            );
            error(Some(name), msg.as_str());
            anyhow::bail!(msg)
        }
    }

    fn ancestor(
        environment: Rc<RefCell<Environment>>,
        distance: usize,
    ) -> Option<Rc<RefCell<Environment>>> {
        let mut env = Some(environment);

        for _ in 0..distance {
            env = env?.borrow().enclosing.clone();
        }

        env
    }

    pub(crate) fn assign(&mut self, name: &Token, value: Value) -> anyhow::Result<()> {
        if self.values.contains_key(&name.lexeme) {
            self.values.insert(name.lexeme.to_string(), Some(value));

            Ok(())
        } else {
            match &mut self.enclosing {
                Some(environment) => environment.borrow_mut().assign(name, value),
                None => {
                    let msg = format!("Undefined variable '{}'.", &name.lexeme);
                    error(Some(name), msg.as_str());
                    anyhow::bail!(msg)
                }
            }
        }
    }
}
