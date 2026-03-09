use crate::Literal;
use logos::Logos;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// An enum representing the possible token types in Lox.
#[derive(Logos, Clone, Debug, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t\r\f]+")] // skip whitespace
#[logos(skip r"//.*")] // skip comments
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
    #[regex(r#""([^"\\]|\\.)*"#)]
    UnterminatedString,
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
    EOF,
}

/// A Lox token.
#[derive(Debug, Clone)]
pub struct Token {
    /// The type of token.
    pub token_type: TokenType,
    /// The lexeme representing the token in the source.
    pub lexeme: String,
    /// The literal value of the token, if it has one.
    pub literal: Option<Literal>,
    /// The line in the source on which the token occurs.
    pub line: usize,
    /// A unique id for each instance.
    id: usize,
}

impl Token {
    /// Create a new token
    pub fn new(
        token_type: TokenType,
        lexeme: String,
        literal: Option<Literal>,
        line: usize,
    ) -> Self {
        Self {
            token_type,
            lexeme,
            literal,
            line,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl PartialEq for Token {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Token {}

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
