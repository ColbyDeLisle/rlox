use crate::tokens::Token;
use crate::{
    compile_time_error,
    expr::Expr,
    interpreter::Interpreter,
    stmt::{Class, Function, If, Return, Stmt, Var, While},
};
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
    Initializer,
    Method,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ClassType {
    None,
    Class,
    SubClass,
}

/// The Lox resolver.
pub(crate) struct Resolver<'a> {
    /// The Lox interpreter for which symbols are being resolved.
    pub(crate) interpreter: &'a mut Interpreter,
    /// A stack of scopes used during resolution.
    scopes: Vec<Scope>,
    /// Whether the current scope is in the body of a callable.
    current_function_type: FunctionType,
    /// Whether the current scope is in a class definition.
    current_class_type: ClassType,
}

impl<'a> Resolver<'a> {
    /// Create a new `Resolver`, provided the `Interpreter` it will resolve for.
    pub(crate) fn new(interpreter: &'a mut Interpreter) -> Self {
        Self {
            interpreter,
            scopes: Vec::new(),
            current_function_type: FunctionType::None,
            current_class_type: ClassType::None,
        }
    }

    /// Resolve symbols in a sequence of statements, updating the interpreter with the results.
    pub(crate) fn resolve(&mut self, stmts: &[Stmt]) -> anyhow::Result<()> {
        let mut had_error = false;

        for stmt in stmts {
            if self.resolve_stmt(stmt).is_err() {
                had_error = true;
            }
        }

        if had_error {
            Err(anyhow::anyhow!("Resolution errors found."))
        } else {
            Ok(())
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) -> anyhow::Result<()> {
        match stmt {
            Stmt::Block(stmts) => {
                self.begin_scope();
                self.resolve(stmts)?;
                self.end_scope();
            }
            Stmt::Class(Class {
                name,
                methods,
                superclass,
            }) => {
                let enclosing_class_type = self.current_class_type;
                self.current_class_type = ClassType::Class;

                self.declare(name)?;
                self.define(name.lexeme.clone());

                if let Some(class) = superclass {
                    match class {
                        Expr::Variable(token) => {
                            if token.lexeme == name.lexeme {
                                let msg = "A class can't inherit from itself.";
                                compile_time_error(Some(token), msg);
                                anyhow::bail!(msg);
                            }
                        }
                        _ => unreachable!(),
                    }

                    self.current_class_type = ClassType::SubClass;
                    self.resolve_expr(class)?;

                    self.begin_scope();
                    self.scopes
                        .last_mut()
                        .unwrap()
                        .insert("super".to_string(), SymbolState::Resolved);
                }

                self.begin_scope();
                self.scopes
                    .last_mut()
                    .unwrap()
                    .insert("this".to_string(), SymbolState::Resolved);

                for method in methods {
                    let Stmt::Function(Function {
                        name: token,
                        params,
                        body,
                    }) = method
                    else {
                        unreachable!();
                    };

                    let function_type = if token.lexeme == "init" {
                        FunctionType::Initializer
                    } else {
                        FunctionType::Method
                    };

                    self.resolve_function(token, params, body, function_type)?;
                }

                self.end_scope();

                if superclass.is_some() {
                    self.end_scope();
                }

                self.current_class_type = enclosing_class_type;
            }
            Stmt::Var(Var { name, initializer }) => {
                self.declare(name)?;
                if let Some(expr) = initializer {
                    self.resolve_expr(expr)?;
                }
                self.define(name.lexeme.clone())
            }
            Stmt::Function(Function { name, params, body }) => {
                self.resolve_function(name, params, body, FunctionType::Function)?;
            }
            Stmt::Expression(expr) => {
                self.resolve_expr(expr)?;
            }
            Stmt::If(If {
                condition,
                then_branch,
                else_branch,
            }) => {
                self.resolve_expr(condition)?;
                self.resolve_stmt(then_branch)?;
                if let Some(stmt) = else_branch {
                    self.resolve_stmt(stmt)?;
                }
            }
            Stmt::Print(expr) => {
                self.resolve_expr(expr)?;
            }
            Stmt::Return(Return { keyword, value }) => {
                match self.current_function_type {
                    FunctionType::None => {
                        let msg = "Can't return from top-level code.";
                        compile_time_error(Some(keyword), msg);
                        anyhow::bail!(msg);
                    }
                    FunctionType::Initializer => {
                        if value.is_some() {
                            let msg = "Can't return a value from an initializer.";
                            compile_time_error(Some(keyword), msg);
                            anyhow::bail!(msg);
                        }
                    }
                    _ => {}
                }

                if let Some(expr) = value {
                    self.resolve_expr(expr)?;
                }
            }
            Stmt::While(While { condition, body }) => {
                self.resolve_expr(condition)?;
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
            if scope.get(&name.lexeme).is_some() {
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
                if let Some(scope) = self.scopes.last_mut()
                    && scope.get(&token.lexeme) == Some(&SymbolState::Pending)
                {
                    let msg = "Can't read local variable in its own initializer.";
                    compile_time_error(Some(token), msg);
                    anyhow::bail!(msg);
                }

                self.resolve_local(token);
            }
            Expr::Assign(assign) => {
                self.resolve_expr(assign.value.as_ref())?;
                self.resolve_local(&assign.name);
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
            Expr::Super(supr) => match self.current_class_type {
                ClassType::None => {
                    let msg = "Can't use 'super' outside of a class.";
                    compile_time_error(Some(&supr.keyword), msg);
                    anyhow::bail!(msg);
                }
                ClassType::Class => {
                    let msg = "Can't use 'super' in a class with no superclass.";
                    compile_time_error(Some(&supr.keyword), msg);
                    anyhow::bail!(msg);
                }
                ClassType::SubClass => {
                    self.resolve_local(&supr.keyword);
                }
            },
            Expr::This(token) => {
                if self.current_class_type == ClassType::None {
                    let msg = "Can't use 'this' outside of a class.";
                    compile_time_error(Some(token), msg);
                    anyhow::bail!(msg);
                }

                self.resolve_local(token);
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

    fn resolve_function(
        &mut self,
        token: &Token,
        params: &[Token],
        body: &[Stmt],
        function_type: FunctionType,
    ) -> anyhow::Result<()> {
        self.declare(token)?;
        self.define(token.lexeme.clone());

        let enclosing_function_type = self.current_function_type;
        self.current_function_type = function_type;
        self.begin_scope();
        for param in params {
            self.declare(param)?;
            self.define(param.lexeme.clone());
        }
        self.resolve(body)?;
        self.end_scope();
        self.current_function_type = enclosing_function_type;

        Ok(())
    }
}
