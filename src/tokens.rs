use crate::Literal;
use logos::Logos;

#[derive(Logos, Clone, Debug, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub literal: Option<Literal>,
    pub line: usize,
}
