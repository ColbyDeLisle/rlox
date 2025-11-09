use crate::{
    Literal, error,
    expr::{Binary, Expr, Grouping, Unary},
    lexer::Lexer,
    tokens::{Token, TokenType, TokenType::*},
};
use std::iter::Peekable;

pub struct Parser<'source> {
    tokens: Peekable<Lexer<'source>>,
    previous: Token,
    had_error: bool,
}

impl<'source> Parser<'source> {
    pub fn new(lexer: Lexer<'source>) -> Option<Self> {
        let mut tokens = lexer.peekable();
        let had_error = false;

        // There is no previous token at the very beginning.
        // We use an Error as a placeholder, but it should never show up anywhere!
        let previous = Token {
            token_type: Error,
            lexeme: "PRE-TOKEN".to_string(),
            literal: None,
            line: 0,
        };

        match tokens.peek() {
            Some(_) => Some(Parser {
                tokens,
                previous,
                had_error,
            }),
            None => {
                error(None, "Cannot create Lexer; no tokens!");
                None
            }
        }
    }

    pub fn parse(&mut self) -> anyhow::Result<Expr> {
        self.expression()
    }

    fn error(&mut self, message: &str) {
        error(Some(&self.previous), message);
        self.had_error = true;
    }

    fn advance(&mut self) {
        if self.tokens.peek().is_some() {
            self.previous = self.tokens.next().unwrap();
        }
    }

    fn matches(&mut self, token_types: &[TokenType]) -> bool {
        for token_type in token_types {
            if self.check(token_type) {
                self.advance();
                return true;
            }
        }

        false
    }

    fn check(&mut self, token_type: &TokenType) -> bool {
        match self.tokens.peek() {
            None => false,
            Some(peeked_token) => peeked_token.token_type == *token_type,
        }
    }

    fn expression(&mut self) -> anyhow::Result<Expr> {
        self.equality()
    }

    fn equality(&mut self) -> anyhow::Result<Expr> {
        let mut expr: Expr = self.comparison()?;

        while self.matches(&[BangEqual, EqualEqual]) {
            let operator: Token = self.previous.clone();
            let right: Expr = self.comparison()?;
            expr = Expr::Binary(Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }

        Ok(expr)
    }

    fn comparison(&mut self) -> anyhow::Result<Expr> {
        let mut expr: Expr = self.term()?;

        while self.matches(&[Greater, GreaterEqual, Less, LessEqual]) {
            let operator: Token = self.previous.clone();
            let right: Expr = self.term()?;
            expr = Expr::Binary(Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }

        Ok(expr)
    }

    fn term(&mut self) -> anyhow::Result<Expr> {
        let mut expr: Expr = self.factor()?;

        while self.matches(&[Minus, Plus]) {
            let operator: Token = self.previous.clone();
            let right: Expr = self.factor()?;
            expr = Expr::Binary(Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }

        Ok(expr)
    }

    fn factor(&mut self) -> anyhow::Result<Expr> {
        let mut expr: Expr = self.unary()?;

        while self.matches(&[Slash, Star]) {
            let operator: Token = self.previous.clone();
            let right: Expr = self.unary()?;
            expr = Expr::Binary(Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }

        Ok(expr)
    }

    fn unary(&mut self) -> anyhow::Result<Expr> {
        if self.matches(&[Bang, Minus]) {
            let operator: Token = self.previous.clone();
            let right: Expr = self.unary()?;
            Ok(Expr::Unary(Unary {
                operator,
                right: Box::new(right),
            }))
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> anyhow::Result<Expr> {
        if self.tokens.peek().is_none() {
            let msg = "Unexpected EOF.";
            self.error(msg);
            anyhow::bail!(msg.to_string());
        }

        // consume the literal, putting it into self.current
        self.advance();

        let p = match &self.previous.token_type {
            True => Expr::Literal(Literal::Bool(true)),
            False => Expr::Literal(Literal::Bool(false)),
            Nil => Expr::Literal(Literal::Nil),
            Number | String => {
                Expr::Literal(self.previous.clone().literal.expect("can get literal"))
            }
            LeftParen => {
                let expr = self.expression()?;
                self.consume(RightParen, "Expect ')' after expression.")?;
                return Ok(Expr::Grouping(Grouping {
                    expression: Box::new(expr),
                }));
            }
            other => {
                let msg = format!("Unexpected token type: {:?}", &other);
                self.error(msg.as_str());
                anyhow::bail!(msg);
            }
        };

        Ok(p)
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> anyhow::Result<bool> {
        if self.check(&token_type) {
            self.advance();
            Ok(true)
        } else {
            self.error(message);
            anyhow::bail!(message.to_string());
        }
    }

    fn synchronize(&mut self) {
        self.advance();

        while let Some(token) = self.tokens.peek() {
            if self.previous.token_type == Semicolon {
                return;
            }

            match token.token_type {
                Class | Fun | Var | For | If | While | Print | Return => {
                    return;
                }
                _ => {}
            };

            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Literal,
        expr::{Binary, Expr, Grouping, Unary},
        lexer::Lexer,
        parser::Parser,
        tokens::{Token, TokenType},
    };

    #[test]
    fn test_parse_expr_simple() {
        let source = "1 + 2";

        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).expect("can create parser");

        let expr = parser.parse().expect("can parse expr");

        let one = Expr::Literal(Literal::Number(1.0));
        let two = Expr::Literal(Literal::Number(2.0));

        let expected_expr = Expr::Binary(Binary {
            left: Box::new(one),
            operator: Token {
                token_type: TokenType::Plus,
                lexeme: "+".to_string(),
                literal: None,
                line: 1,
            },
            right: Box::new(two),
        });

        assert_eq!(expr, expected_expr);
    }

    #[test]
    fn test_parse_expr_nested() {
        let source = r#"
            -4 * (3 / (1 + 2))
        "#;

        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).expect("can create parser");

        let expr = parser.parse().expect("can parse expr");

        let one = Expr::Literal(Literal::Number(1.0));
        let two = Expr::Literal(Literal::Number(2.0));
        let three = Expr::Literal(Literal::Number(3.0));
        let minus_four = Expr::Unary(Unary {
            operator: Token {
                token_type: TokenType::Minus,
                lexeme: "-".to_string(),
                literal: None,
                line: 2,
            },
            right: Box::new(Expr::Literal(Literal::Number(4.0))),
        });

        let one_plus_two = Expr::Grouping(Grouping {
            expression: Box::new(Expr::Binary(Binary {
                left: Box::new(one),
                operator: Token {
                    token_type: TokenType::Plus,
                    lexeme: "+".to_string(),
                    literal: None,
                    line: 2,
                },
                right: Box::new(two),
            })),
        });

        let three_over_oneplustwo = Expr::Grouping(Grouping {
            expression: Box::new(Expr::Binary(Binary {
                left: Box::new(three),
                operator: Token {
                    token_type: TokenType::Slash,
                    lexeme: "/".to_string(),
                    literal: None,
                    line: 2,
                },
                right: Box::new(one_plus_two),
            })),
        });

        let expected_expr = Expr::Binary(Binary {
            left: Box::new(minus_four),
            operator: Token {
                token_type: TokenType::Star,
                lexeme: "*".to_string(),
                literal: None,
                line: 2,
            },
            right: Box::new(three_over_oneplustwo),
        });

        assert_eq!(expr, expected_expr);
    }

    #[test]
    fn test_parse_expr_err() {
        // same as above, with missing right paren
        let source = r#"
            4 * (3 / (1 + 2)
        "#;

        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).expect("can create parser");

        let result = parser.parse();

        assert!(result.is_err());
        assert!(parser.had_error);
    }

    #[test]
    fn test_cannot_create_parser_with_empty_lexer() {
        let source = "";

        let lexer = Lexer::new(source);

        assert!(Parser::new(lexer).is_none());
    }
}
