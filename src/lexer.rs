use crate::Literal;
use crate::tokens::{Token, TokenType};

pub struct Lexer<'source> {
    logos_lexer: logos::Lexer<'source, TokenType>,
    source: &'source str,
    line: usize,
}

impl<'source> Lexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            logos_lexer: logos::Lexer::<TokenType>::new(source),
            source,
            line: 1,
        }
    }

    pub fn next_token(&mut self) -> Option<Token> {
        loop {
            let result = self.logos_lexer.next()?;

            let tok = match result {
                Ok(tok) => tok,
                Err(_) => {
                    return Some(Token {
                        token_type: TokenType::Error,
                        lexeme: self.source[self.logos_lexer.span()].to_string(),
                        literal: None,
                        line: self.line,
                    });
                }
            };

            let span = self.logos_lexer.span();
            let lexeme = &self.source[span.clone()];

            if tok == TokenType::NewLine  {
                self.line += lexeme.matches('\n').count();
                // these tokens are only used for line counting
                continue;
            }

            let literal = match tok {
                TokenType::Number => {
                    let val = lexeme.parse::<f32>().unwrap();
                    Some(Literal::Number(val))
                }
                TokenType::String => {
                    let val = lexeme[1..lexeme.len() - 1].to_string(); // strip quotes
                    self.line += val.matches("\n").count();
                    Some(Literal::String(val))
                }
                TokenType::Identifier => {
                    let val = lexeme.to_string();
                    Some(Literal::Identifier(val))
                }
                _ => None,
            };

            return Some(Token {
                token_type: tok,
                lexeme: lexeme.to_string(),
                literal,
                line: self.line,
            });
        }
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}
