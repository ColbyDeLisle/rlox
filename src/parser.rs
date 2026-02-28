use crate::expr::Logical;
use crate::{
    Literal, error,
    expr::{Assign, Binary, Call, Expr, Grouping, Unary},
    lexer::Lexer,
    stmt::Stmt,
    tokens::{Token, TokenType, TokenType::*},
};
use std::fmt::{Display, Formatter};
use std::iter::Peekable;

#[derive(Debug)]
enum FunctionKind {
    Function,
    #[allow(dead_code)]
    Method,
}

impl Display for FunctionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionKind::Function => write!(f, "function"),
            FunctionKind::Method => write!(f, "method"),
        }
    }
}

/// The Lox parser.
pub struct Parser<'source> {
    /// The stream of tokens used to parse, coming dynamically from the provided lexer.
    tokens: Peekable<Lexer<'source>>,
    /// The previously processed `Token`.
    previous: Token,
    /// Whether the parser has so far encountered an error.
    had_error: bool,
}

impl<'source> Parser<'source> {
    /// Create a new `Parser`, provided a `Lexer`.
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

    /// Parse the provided source code into a sequence of statements.
    pub fn parse(&mut self) -> anyhow::Result<Vec<Stmt>> {
        let mut stmts: Vec<Stmt> = vec![];

        while self.tokens.peek().is_some() {
            stmts.push(self.decl()?);
        }

        Ok(stmts)
    }

    fn decl(&mut self) -> anyhow::Result<Stmt> {
        let stmt: anyhow::Result<Stmt> = if self.matches(&[Var]) {
            self.var_decl()
        } else if self.matches(&[Fun]) {
            self.fun_decl(FunctionKind::Function)
        } else {
            self.stmt()
        };

        if stmt.is_err() {
            self.synchronize();
        }

        stmt
    }

    fn var_decl(&mut self) -> anyhow::Result<Stmt> {
        self.consume(Identifier, "Expect variable name.")?;
        let name = self.previous.clone();

        let mut initializer: Option<Expr> = None;
        if self.matches(&[Equal]) {
            initializer = Some(self.expr()?);
        }

        self.consume(Semicolon, "Expect ';' after variable declaration.")?;

        Ok(Stmt::Var(name, initializer))
    }

    fn fun_decl(&mut self, kind: FunctionKind) -> anyhow::Result<Stmt> {
        self.consume(Identifier, &format!("Expect {kind} name."))?;
        let name = self.previous.clone();

        self.consume(LeftParen, &format!("Expect '(' after {kind} name."))?;
        let mut params: Vec<Token> = vec![];
        if !self.check(&RightParen) {
            self.consume(Identifier, "Expect parameter name.")?;
            params.push(self.previous.clone());

            while self.matches(&[Comma]) {
                if params.len() >= 255 {
                    self.error("Can't have more than 255 parameters.");
                }

                self.consume(Identifier, "Expect parameter name.")?;
                params.push(self.previous.clone());
            }
        }
        self.consume(RightParen, &format!("Expect ')' after {kind} parameters."))?;

        self.consume(LeftBrace, &format!("Expect '{{' before {kind} body."))?;
        // n.b., we consume the LeftBrace *before* calling block
        let body = self.block()?;
        let Stmt::Block(stmts) = body else {
            let msg = "Calling block did not return a Block statement. This should never happen.";
            self.error(msg);
            anyhow::bail!(msg.to_string());
        };

        Ok(Stmt::Function(name, params, stmts))
    }

    fn stmt(&mut self) -> anyhow::Result<Stmt> {
        if self.matches(&[If]) {
            self.if_stmt()
        } else if self.matches(&[While]) {
            self.while_stmt()
        } else if self.matches(&[For]) {
            self.for_stmt()
        } else if self.matches(&[LeftBrace]) {
            self.block()
        } else if self.matches(&[Print]) {
            self.print_stmt()
        } else if self.matches(&[Return]) {
            self.return_stmt()
        } else {
            self.expr_stmt()
        }
    }

    fn if_stmt(&mut self) -> anyhow::Result<Stmt> {
        self.consume(LeftParen, "Expect '(' after 'if'.")?;
        let condition = self.expr()?;
        self.consume(RightParen, "Expect ')' after condition.")?;

        let then_branch = Box::new(self.stmt()?);
        let else_branch = if self.matches(&[Else]) {
            Some(Box::new(self.stmt()?))
        } else {
            None
        };

        Ok(Stmt::If(condition, then_branch, else_branch))
    }

    fn while_stmt(&mut self) -> anyhow::Result<Stmt> {
        self.consume(LeftParen, "Expect '(' after 'while'.")?;
        let condition = self.expr()?;
        self.consume(RightParen, "Expect ')' after condition.")?;
        let body = self.stmt()?;

        Ok(Stmt::While(condition, Box::new(body)))
    }

    fn for_stmt(&mut self) -> anyhow::Result<Stmt> {
        self.consume(LeftParen, "Expect '(' after 'for'.")?;

        let initializer = if self.matches(&[Semicolon]) {
            None
        } else if self.matches(&[Var]) {
            Some(self.var_decl()?)
        } else {
            Some(self.expr_stmt()?)
        };

        let condition = if self.check(&Semicolon) {
            None
        } else {
            Some(self.expr()?)
        };
        self.consume(Semicolon, "Expect ';' after loop condition.")?;

        let increment = if self.check(&RightParen) {
            None
        } else {
            Some(self.expr()?)
        };

        self.consume(RightParen, "Expect ')' after for clauses.")?;

        let mut body = self.stmt()?;

        if let Some(increment) = increment {
            body = Stmt::Block(vec![body, Stmt::Expression(increment)]);
        }

        body = Stmt::While(
            condition.unwrap_or(Expr::Literal(Literal::Bool(true))),
            Box::new(body),
        );

        if let Some(initializer) = initializer {
            body = Stmt::Block(vec![initializer, body]);
        }

        Ok(body)
    }

    fn block(&mut self) -> anyhow::Result<Stmt> {
        let mut statements: Vec<Stmt> = vec![];

        while !self.check(&RightBrace) & self.tokens.peek().is_some() {
            statements.push(self.decl()?);
        }
        self.consume(RightBrace, "Expect '}' after block.")?;

        Ok(Stmt::Block(statements))
    }

    fn print_stmt(&mut self) -> anyhow::Result<Stmt> {
        let value = self.expr()?;
        self.consume(Semicolon, "Expect ';' after value.")?;

        Ok(Stmt::Print(value))
    }

    fn expr_stmt(&mut self) -> anyhow::Result<Stmt> {
        let expr = self.expr()?;
        self.consume_without_error(Semicolon); //, "Expect ';' after expression.")?;

        Ok(Stmt::Expression(expr))
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

    pub(crate) fn expr(&mut self) -> anyhow::Result<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> anyhow::Result<Expr> {
        let l_expr = self.logical_or()?;

        if self.matches(&[Equal]) {
            let value = self.assignment()?;

            match l_expr {
                Expr::Variable(token) => {
                    return Ok(Expr::Assign(Assign {
                        name: token,
                        value: Box::new(value),
                    }));
                }
                _ => {
                    self.error("Invalid assignment target.");
                }
            }
        }

        Ok(l_expr)
    }

    fn logical_or(&mut self) -> anyhow::Result<Expr> {
        let mut expr: Expr = self.logical_and()?;

        while self.matches(&[Or]) {
            let operator: Token = self.previous.clone();
            let right: Expr = self.logical_and()?;
            expr = Expr::Logical(Logical {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }

        Ok(expr)
    }

    fn logical_and(&mut self) -> anyhow::Result<Expr> {
        let mut expr: Expr = self.equality()?;

        while self.matches(&[And]) {
            let operator: Token = self.previous.clone();
            let right: Expr = self.equality()?;
            expr = Expr::Logical(Logical {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            });
        }

        Ok(expr)
    }

    fn equality(&mut self) -> anyhow::Result<Expr> {
        let mut expr: Expr = self.comparison()?;

        while self.matches(&[BangEqual, EqualEqual]) {
            let operator: Token = self.previous.clone();
            let right: Expr = self.comparison()?;
            expr = Expr::Binary(Binary {
                left: Box::new(expr.clone()),
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
            self.call()
        }
    }

    fn call(&mut self) -> anyhow::Result<Expr> {
        let mut expr = self.primary()?;

        loop {
            if self.matches(&[LeftParen]) {
                expr = self.finish_call(expr.clone())?;
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> anyhow::Result<Expr> {
        let mut args: Vec<Expr> = vec![];

        if !self.check(&RightParen) {
            args.push(self.expr()?);
            while self.matches(&[Comma]) {
                if args.len() > 255 {
                    self.error("Functions may not have more than 255 arguments.");
                }
                args.push(self.expr()?);
            }
        }

        self.consume(RightParen, "Expect ')' after args.")?;

        Ok(Expr::Call(Call {
            callee: Box::new(callee),
            paren: self.previous.clone(),
            args,
        }))
    }

    fn return_stmt(&mut self) -> anyhow::Result<Stmt> {
        let keyword = self.previous.clone();

        let mut value = None;
        if !self.check(&Semicolon) {
            value = Some(self.expr()?);
        }

        self.consume(Semicolon, "Expect ; after return value.")?;

        Ok(Stmt::Return(keyword, value))
    }

    fn primary(&mut self) -> anyhow::Result<Expr> {
        if self.tokens.peek().is_none() {
            let msg = "Unexpected EOF.";
            self.error(msg);
            anyhow::bail!(msg.to_string());
        }

        // consume the literal
        self.advance();

        let p = match &self.previous.token_type {
            True => Expr::Literal(Literal::Bool(true)),
            False => Expr::Literal(Literal::Bool(false)),
            Nil => Expr::Literal(Literal::Nil),
            Number | String => {
                Expr::Literal(self.previous.clone().literal.expect("can get literal"))
            }
            Identifier => Expr::Variable(self.previous.clone()),
            LeftParen => {
                let expr = self.expr()?;
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

    fn consume_without_error(&mut self, token_type: TokenType) {
        if self.check(&token_type) {
            self.advance();
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
    // Many of these tests use the lexer for convenience. For proper unit testing, one really ought
    // to factor out the other parts of the crate and e.g. prepare sequences of Tokens manually.

    use crate::expr::Assign;
    use crate::stmt::Stmt;
    use crate::{
        Literal,
        expr::{Binary, Expr, Grouping, Logical, Unary},
        lexer::Lexer,
        parser::Parser,
        tokens::{Token, TokenType},
    };

    #[test]
    fn test_parse_expr_simple() {
        let source = "1 + 2";

        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).expect("can create parser");

        let expr = parser.expr().expect("can parse expr");

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

        let expr = parser.expr().expect("can parse expr");

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
    fn test_parse_logical() {
        let source = "5 and true or false";

        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).expect("can create parser");

        let expr = parser.expr().expect("can parse expr");

        let five = Expr::Literal(Literal::Number(5.0));
        let tru = Expr::Literal(Literal::Bool(true));
        let fls = Expr::Literal(Literal::Bool(false));
        let five_and_tru = Expr::Logical(Logical {
            left: Box::new(five),
            operator: Token {
                token_type: TokenType::And,
                lexeme: "and".to_string(),
                literal: None,
                line: 1,
            },
            right: Box::new(tru),
        });

        let expected_expr = Expr::Logical(Logical {
            left: Box::new(five_and_tru),
            operator: Token {
                token_type: TokenType::Or,
                lexeme: "or".to_string(),
                literal: None,
                line: 1,
            },
            right: Box::new(fls),
        });

        assert_eq!(expr, expected_expr);
    }

    #[test]
    fn test_parse_if() {
        let source = r#"
            if (1 == 2) {
                print "help!";
            }
        "#;

        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).expect("can create parser");

        let stmt = parser.stmt().expect("can parse expr");

        let one = Expr::Literal(Literal::Number(1.0));
        let two = Expr::Literal(Literal::Number(2.0));
        let one_equal_two = Expr::Binary(Binary {
            left: Box::new(one),
            operator: Token {
                token_type: TokenType::EqualEqual,
                lexeme: "==".to_string(),
                literal: None,
                line: 2,
            },
            right: Box::new(two),
        });

        let help = Expr::Literal(Literal::String(String::from("help!")));
        let print_help = Stmt::Print(help);
        let block_print_help = Stmt::Block(vec![print_help]);

        let expected_stmt = Stmt::If(one_equal_two, Box::new(block_print_help), None);

        assert_eq!(stmt, expected_stmt);
    }

    #[test]
    fn test_parse_if_else() {
        let source = r#"
            if (1 == 2) {
                print "help!";
            }
            else {
                print "nvm.";
            }
        "#;

        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).expect("can create parser");

        let stmt = parser.stmt().expect("can parse expr");

        let one = Expr::Literal(Literal::Number(1.0));
        let two = Expr::Literal(Literal::Number(2.0));
        let one_equal_two = Expr::Binary(Binary {
            left: Box::new(one),
            operator: Token {
                token_type: TokenType::EqualEqual,
                lexeme: "==".to_string(),
                literal: None,
                line: 2,
            },
            right: Box::new(two),
        });

        let help = Expr::Literal(Literal::String(String::from("help!")));
        let print_help = Stmt::Print(help);
        let block_print_help = Stmt::Block(vec![print_help]);

        let nvm = Expr::Literal(Literal::String(String::from("nvm.")));
        let print_nvm = Stmt::Print(nvm);
        let block_print_nvm = Stmt::Block(vec![print_nvm]);

        let expected_stmt = Stmt::If(
            one_equal_two,
            Box::new(block_print_help),
            Some(Box::new(block_print_nvm)),
        );

        assert_eq!(stmt, expected_stmt);
    }

    #[test]
    fn test_parse_while() {
        let source = r#"
            var i = 0;
            while (i < 10) {
                print i;
                i = i + 1;
            }
        "#;

        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).expect("can create parser");

        let stmts = parser.parse().expect("can parse source");

        assert_eq!(stmts.len(), 2);
        // don't care so much about the declaration here
        assert!(matches!(&stmts[0], Stmt::Var(_, _)));

        fn i_at_line(line: usize) -> Token {
            Token {
                token_type: TokenType::Identifier,
                lexeme: String::from("i"),
                literal: Some(Literal::String(String::from("i"))),
                line,
            }
        }

        let one = Expr::Literal(Literal::Number(1.0));
        let ten = Expr::Literal(Literal::Number(10.0));
        let i_less_than_ten = Expr::Binary(Binary {
            left: Box::new(Expr::Variable(i_at_line(3))),
            operator: Token {
                token_type: TokenType::Less,
                lexeme: String::from("<"),
                literal: None,
                line: 3,
            },
            right: Box::new(ten),
        });
        let print_i = Stmt::Print(Expr::Variable(i_at_line(4)));
        let i_plus_one = Expr::Binary(Binary {
            left: Box::new(Expr::Variable(i_at_line(5))),
            operator: Token {
                token_type: TokenType::Plus,
                lexeme: String::from("+"),
                literal: None,
                line: 5,
            },
            right: Box::new(one),
        });
        let i_equals_i_plus_one = Stmt::Expression(Expr::Assign(Assign {
            name: i_at_line(5),
            value: Box::new(i_plus_one),
        }));

        let while_loop = Stmt::While(
            i_less_than_ten,
            Box::new(Stmt::Block(vec![print_i, i_equals_i_plus_one])),
        );

        assert_eq!(stmts[1], while_loop);
    }

    #[test]
    fn test_parse_expr_err() {
        // missing right paren
        let source = r#"
            4 * (3 / (1 + 2)
        "#;

        let lexer = Lexer::new(source);
        let mut parser = Parser::new(lexer).expect("can create parser");

        let result = parser.expr();

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
