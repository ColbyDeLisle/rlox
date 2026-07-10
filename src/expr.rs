use crate::Literal;
use crate::tokens::Token;

/// A Lox expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Array(Vec<Expr>),
    Assign(Assign),
    Binary(Binary),
    Call(Call),
    Get(Get),
    Grouping(Grouping),
    Index(Index),
    IndexSet(IndexSet),
    Literal(Literal),
    Logical(Logical),
    Set(Set),
    Super(Super),
    This(Token),
    Unary(Unary),
    Variable(Token),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assign {
    pub name: Token,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binary {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call {
    pub callee: Box<Expr>,
    pub paren: Token,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grouping {
    pub expression: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Logical {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unary {
    pub operator: Token,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Get {
    pub expr: Box<Expr>,
    pub name: Token,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Set {
    pub expr: Box<Expr>,
    pub name: Token,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Super {
    pub keyword: Token,
    pub method: Token,
}

/// An index read, e.g. `a[i]`. The `bracket` token is retained for error reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    pub object: Box<Expr>,
    pub bracket: Token,
    pub index: Box<Expr>,
}

/// An index assignment, e.g. `a[i] = value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexSet {
    pub object: Box<Expr>,
    pub bracket: Token,
    pub index: Box<Expr>,
    pub value: Box<Expr>,
}
