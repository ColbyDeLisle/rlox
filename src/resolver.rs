use std::collections::HashMap;
use crate::{interpreter::Interpreter, stmt::Stmt, expr::Expr, error};
use crate::tokens::Token;

type Scope = HashMap<String, bool>;

pub(crate) struct Resolver {
    pub(crate) interpreter: Interpreter,
    scopes: Vec<Scope>
}

impl Resolver {
    pub(crate) fn new(interpreter: Interpreter) -> Self {
        Self {
            interpreter,
            scopes: Vec::new()
        }
    }

    pub(crate) fn resolve(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.resolve_stmt(stmt);
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Block(stmts) => {
                self.begin_scope();
                self.resolve(stmts);
                self.end_scope();
            },
            Stmt::Var(token, expr) => {
                self.declare(token.lexeme.clone());
                if let Some(initializer) = expr {
                    self.resolve_expr(&initializer);
                }
                self.define(token.lexeme.clone())
            },
            Stmt::Function(token, params, body) => {
                self.declare(token.lexeme.clone());
                self.define(token.lexeme.clone());

                self.begin_scope();
                for param in params {
                    self.declare(param.lexeme.clone());
                    self.define(param.lexeme.clone());
                }
                self.resolve(body);
                self.end_scope();
            }
            Stmt::Expression(expr) => {
                self.resolve_expr(&expr);
            }
            Stmt::If(condition , if_body, else_body) => {
                self.resolve_expr(&condition);
                self.resolve_stmt(if_body);
                if let Some(stmt) = else_body {
                    self.resolve_stmt(stmt);
                }
            }
            Stmt::Print(expr) => {
                self.resolve_expr(&expr);
            }
            Stmt::Return(_, expr) => {
                if let Some(expr) = expr {
                    self.resolve_expr(&expr);
                }
            }
            Stmt::While(condition, body) => {
                self.resolve_expr(&condition);
                self.resolve_stmt(body);
            }
        }
    }

    fn begin_scope(&mut self) {
        self.scopes.push(Scope::new())
    }

    fn end_scope(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }

    fn declare(&mut self, name: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, false);
        }
    }

    fn define(&mut self, name: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, true);
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Variable(token) => {
                if let Some(scope) = self.scopes.last_mut() {
                    if scope.get(&token.lexeme) == Some(&false) {
                        error(Some(&token), "Can't read local variable in its own initializer.");
                    }
                }

                self.resolve_local(token);
            }
            Expr::Assign(assign) => {
                self.resolve_expr(assign.value.as_ref());
                self.resolve_local(&assign.name)
            }
            Expr::Binary(binary) => {
                self.resolve_expr(binary.left.as_ref());
                self.resolve_expr(binary.right.as_ref());
            }
            Expr::Call(call) => {
                self.resolve_expr(call.callee.as_ref());
                for arg in &call.args {
                    self.resolve_expr(arg);
                }
            }
            Expr::Grouping(grouping) => {
                self.resolve_expr(grouping.expression.as_ref());
            }
            Expr::Literal(_) => {}
            Expr::Logical(logical) => {
                self.resolve_expr(logical.left.as_ref());
                self.resolve_expr(logical.right.as_ref());
            }
            Expr::Unary(unary) => {
                self.resolve_expr(unary.right.as_ref());
            }
        }
    }

    fn resolve_local(&mut self, name: &Token) {
        for i in (0..self.scopes.len()).rev() {
            if self.scopes.get(i).unwrap().contains_key(&name.lexeme) {
                self.interpreter.resolve(name.clone(), self.scopes.len() - 1 - i);
                break;
            }
        }
    }
}
