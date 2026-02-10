use crate::Literal;
use logos::Logos;
use std::hash::{Hash, Hasher};

#[derive(Logos, Clone, Debug, PartialEq, Eq, Hash)]
// Skip whitespace and comments
#[logos(skip r"[ \t\r\f]+")]
#[logos(skip r"//.*")]
pub enum TokenType {
    // Single-character tokens
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token(",")]
    Comma,
    #[token(".")]
    Dot,
    #[token("-")]
    Minus,
    #[token("+")]
    Plus,
    #[token(";")]
    Semicolon,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,

    // One or two character tokens
    #[token("!")]
    Bang,
    #[token("!=")]
    BangEqual,
    #[token("=")]
    Equal,
    #[token("==")]
    EqualEqual,
    #[token("<")]
    Less,
    #[token("<=")]
    LessEqual,
    #[token(">")]
    Greater,
    #[token(">=")]
    GreaterEqual,

    // Literals
    #[regex(r#"[0-9]+(\.[0-9]+)?"#)]
    Number,
    #[regex(r#""([^"\\]|\\.)*""#)]
    String,
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*")]
    Identifier,

    // Keywords
    #[token("and")]
    And,
    #[token("class")]
    Class,
    #[token("else")]
    Else,
    #[token("false")]
    False,
    #[token("fun")]
    Fun,
    #[token("for")]
    For,
    #[token("if")]
    If,
    #[token("nil")]
    Nil,
    #[token("or")]
    Or,
    #[token("print")]
    Print,
    #[token("return")]
    Return,
    #[token("super")]
    Super,
    #[token("this")]
    This,
    #[token("true")]
    True,
    #[token("var")]
    Var,
    #[token("while")]
    While,

    // New line (used for line numbering)
    #[regex(r"[\n]+")]
    NewLine,

    Error,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub literal: Option<Literal>,
    pub line: usize,
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        (self.token_type == other.token_type) && (self.lexeme == other.lexeme) && (self.line == other.line)
    }
}

impl Eq for Token {}

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.token_type.hash(state);
        self.lexeme.hash(state);
        self.line.hash(state);
    }
}
