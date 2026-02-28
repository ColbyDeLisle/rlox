use crate::Literal;
use crate::tokens::Token;
use std::fmt::{Display, Formatter};

/// A Lox expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Assign(Assign),
    Binary(Binary),
    Call(Call),
    Grouping(Grouping),
    Literal(Literal),
    Logical(Logical),
    Unary(Unary),
    Variable(Token), // here I'm not using a separate struct for the data
}

// impl Display for Expr {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Expr::Assign(assign) => write!(f, "{assign}"),
//             Expr::Binary(binary) => write!(f, "{binary}"),
//             Expr::Call(call) => write!(f, "{call}"),
//             Expr::Grouping(grouping) => write!(f, "{grouping}"),
//             Expr::Literal(literal) => write!(f, "{literal}"),
//             Expr::Logical(logical) => write!(f, "{logical}"),
//             Expr::Unary(unary) => write!(f, "{unary}"),
//             Expr::Variable(token) => write!(f, "{}", token.lexeme),
//         }
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    pub name: Token,
    pub value: Box<Expr>,
}

// impl Display for Assign {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{} = {};", self.name.lexeme, self.value)
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct Binary {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

// impl Display for Binary {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "({} {} {})", self.operator.lexeme, self.left, self.right)
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub callee: Box<Expr>,
    pub paren: Token,
    pub args: Vec<Expr>,
}

// impl Display for Call {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{} ({:?})", self.callee, &self.args)
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct Grouping {
    pub expression: Box<Expr>,
}

// impl Display for Grouping {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "({})", self.expression)
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct Logical {
    pub left: Box<Expr>,
    pub operator: Token,
    pub right: Box<Expr>,
}

// impl Display for Logical {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "({} {} {})", self.operator.lexeme, self.left, self.right)
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct Unary {
    pub operator: Token,
    pub right: Box<Expr>,
}

// impl Display for Unary {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "({} {})", self.operator.lexeme, self.right)
//     }
// }
