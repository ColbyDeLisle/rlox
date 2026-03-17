use crate::{Token, expr::Expr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    If(If),
    While(While),
    Block(Vec<Stmt>),
    Class(Class),
    Expression(Expr),
    Var(Var),
    Print(Expr),
    Function(Function),
    Return(Return),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct If {
    pub(crate) condition: Expr,
    pub(crate) then_branch: Box<Stmt>,
    pub(crate) else_branch: Option<Box<Stmt>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct While {
    pub(crate) condition: Expr,
    pub(crate) body: Box<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub(crate) name: Token,
    pub(crate) methods: Vec<Stmt>,
    pub(crate) superclass: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Var {
    pub(crate) name: Token,
    pub(crate) initializer: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub(crate) name: Token,
    pub(crate) params: Vec<Token>,
    pub(crate) body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Return {
    pub(crate) keyword: Token,
    pub(crate) value: Option<Expr>,
}
