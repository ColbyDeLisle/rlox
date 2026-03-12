use crate::tokens::Token;
use crate::{compile_time_error, expr::Expr, interpreter::Interpreter, stmt::Stmt};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymbolState {
    Pending,
    Resolved,
}

type Scope = HashMap<String, SymbolState>;

#[derive(Debug, Clone, Copy, PartialEq)]
enum FunctionType {
    None,
    Function,
}

/// The Lox resolver.
pub(crate) struct Resolver {
    /// The Lox interpreter for which symbols are being resolved.
    pub(crate) interpreter: Interpreter,
    /// A stack of scopes used during resolution.
    scopes: Vec<Scope>,
    /// Whether the current scope is the body of a callable.
    current_function_type: FunctionType,
}

impl Resolver {
    /// Create a new `Resolver`, provided the `Interpreter` it will resolve for.
    pub(crate) fn new(interpreter: Interpreter) -> Self {
        Self {
            interpreter,
            scopes: Vec::new(),
            current_function_type: FunctionType::None,
        }
    }

    /// Resolve symbols in a sequence of statements, updating the interpreter with the results.
    pub(crate) fn resolve(&mut self, stmts: &[Stmt]) -> anyhow::Result<()> {
        for stmt in stmts {
            self.resolve_stmt(stmt)?;
        }

        Ok(())
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) -> anyhow::Result<()> {
        match stmt {
            Stmt::Block(stmts) => {
                self.begin_scope();
                self.resolve(stmts)?;
                self.end_scope();
            }
            Stmt::Class(name, methods) => {
                self.declare(name)?;
                self.define(name.lexeme.clone());
            }
            Stmt::Var(token, expr) => {
                self.declare(token)?;
                if let Some(initializer) = expr {
                    self.resolve_expr(&initializer)?;
                }
                self.define(token.lexeme.clone())
            }
            Stmt::Function(token, params, body) => {
                self.declare(token)?;
                self.define(token.lexeme.clone());

                let enclosing_function_type = self.current_function_type;
                self.current_function_type = FunctionType::Function;
                self.begin_scope();
                for param in params {
                    self.declare(param)?;
                    self.define(param.lexeme.clone());
                }
                self.resolve(body)?;
                self.end_scope();
                self.current_function_type = enclosing_function_type;
            }
            Stmt::Expression(expr) => {
                self.resolve_expr(&expr)?;
            }
            Stmt::If(condition, if_body, else_body) => {
                self.resolve_expr(&condition)?;
                self.resolve_stmt(if_body)?;
                if let Some(stmt) = else_body {
                    self.resolve_stmt(stmt)?;
                }
            }
            Stmt::Print(expr) => {
                self.resolve_expr(&expr)?;
            }
            Stmt::Return(keyword, expr) => {
                if self.current_function_type == FunctionType::None {
                    let msg = "Can't return from top-level code.";
                    compile_time_error(Some(keyword), msg);
                    anyhow::bail!(msg);
                }

                if let Some(expr) = expr {
                    self.resolve_expr(&expr)?;
                }
            }
            Stmt::While(condition, body) => {
                self.resolve_expr(&condition)?;
                self.resolve_stmt(body)?;
            }
        }

        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scopes.push(Scope::new())
    }

    fn end_scope(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }

    fn declare(&mut self, name: &Token) -> anyhow::Result<()> {
        if let Some(scope) = self.scopes.last_mut() {
            if let Some(_) = scope.get(&name.lexeme) {
                let msg = "Already a variable with this name in this scope.";
                compile_time_error(Some(name), msg);
                anyhow::bail!(msg);
            }
            scope.insert(name.lexeme.clone(), SymbolState::Pending);
        }

        Ok(())
    }

    fn define(&mut self, name: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, SymbolState::Resolved);
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) -> anyhow::Result<()> {
        match expr {
            Expr::Variable(token) => {
                if let Some(scope) = self.scopes.last_mut() {
                    if scope.get(&token.lexeme) == Some(&SymbolState::Pending) {
                        let msg = "Can't read local variable in its own initializer.";
                        compile_time_error(Some(&token), msg);
                        anyhow::bail!(msg);
                    }
                }

                self.resolve_local(token);
            }
            Expr::Assign(assign) => {
                self.resolve_expr(assign.value.as_ref())?;
                self.resolve_local(&assign.name)
            }
            Expr::Binary(binary) => {
                self.resolve_expr(binary.left.as_ref())?;
                self.resolve_expr(binary.right.as_ref())?;
            }
            Expr::Call(call) => {
                self.resolve_expr(call.callee.as_ref())?;
                for arg in &call.args {
                    self.resolve_expr(arg)?;
                }
            }
            Expr::Get(get) => {
                self.resolve_expr(get.expr.as_ref())?;
            }
            Expr::Grouping(grouping) => {
                self.resolve_expr(grouping.expression.as_ref())?;
            }
            Expr::Literal(_) => {}
            Expr::Logical(logical) => {
                self.resolve_expr(logical.left.as_ref())?;
                self.resolve_expr(logical.right.as_ref())?;
            }
            Expr::Set(set) => {
                self.resolve_expr(set.value.as_ref())?;
                self.resolve_expr(set.expr.as_ref())?;
            }
            Expr::Unary(unary) => {
                self.resolve_expr(unary.right.as_ref())?;
            }
        }

        Ok(())
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
