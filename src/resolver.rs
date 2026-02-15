use crate::tokens::Token;
use crate::{error, expr::Expr, interpreter::Interpreter, stmt::Stmt};
use std::collections::HashMap;

type Scope = HashMap<String, bool>;

#[derive(Debug, Clone, Copy, PartialEq)]
enum FunctionType {
    None,
    Function,
}

pub(crate) struct Resolver {
    pub(crate) interpreter: Interpreter,
    scopes: Vec<Scope>,
    current_function_type: FunctionType,
}

impl Resolver {
    pub(crate) fn new(interpreter: Interpreter) -> Self {
        Self {
            interpreter,
            scopes: Vec::new(),
            current_function_type: FunctionType::None,
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
            }
            Stmt::Var(token, expr) => {
                self.declare(token);
                if let Some(initializer) = expr {
                    self.resolve_expr(&initializer);
                }
                self.define(token.lexeme.clone())
            }
            Stmt::Function(token, params, body) => {
                self.declare(token);
                self.define(token.lexeme.clone());

                let enclosing_function_type = self.current_function_type;
                self.current_function_type = FunctionType::Function;
                self.begin_scope();
                for param in params {
                    self.declare(param);
                    self.define(param.lexeme.clone());
                }
                self.resolve(body);
                self.end_scope();
                self.current_function_type = enclosing_function_type;
            }
            Stmt::Expression(expr) => {
                self.resolve_expr(&expr);
            }
            Stmt::If(condition, if_body, else_body) => {
                self.resolve_expr(&condition);
                self.resolve_stmt(if_body);
                if let Some(stmt) = else_body {
                    self.resolve_stmt(stmt);
                }
            }
            Stmt::Print(expr) => {
                self.resolve_expr(&expr);
            }
            Stmt::Return(keyword, expr) => {
                if self.current_function_type == FunctionType::None {
                    error(Some(keyword), "Can't return from top-level code.");
                }

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

    fn declare(&mut self, name: &Token) {
        if let Some(scope) = self.scopes.last_mut() {
            if let Some(_) = scope.get(&name.lexeme) {
                error(
                    Some(name),
                    "Already a variable with this name in this scope.",
                );
            }
            scope.insert(name.lexeme.clone(), false);
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
                        error(
                            Some(&token),
                            "Can't read local variable in its own initializer.",
                        );
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
                self.interpreter
                    .resolve(name.clone(), self.scopes.len() - 1 - i);
                break;
            }
        }
    }
}
