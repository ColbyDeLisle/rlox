use crate::expr::{Get, Logical, Set};
use crate::{
    Literal, compile_time_error,
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
    pub fn new(lexer: Lexer<'source>) -> Self {
        let tokens = lexer.peekable();
        let had_error = false;

        // There is no previous token at the very beginning.
        // We use an Error as a placeholder, but it should never show up anywhere!
        let previous = Token::new(Error, "PRE-TOKEN".to_string(), None, 0);

        Parser {
            tokens,
            previous,
            had_error,
        }
    }

    /// Parse the provided source code into a sequence of statements.
    pub fn parse(&mut self) -> anyhow::Result<Vec<Stmt>> {
        let mut stmts: Vec<Stmt> = vec![];

        while self.tokens.peek().is_some() {
            if let Some(stmt) = self.decl() {
                stmts.push(stmt);
            }
        }

        if self.had_error {
            anyhow::bail!("");
        }

        Ok(stmts)
    }

    fn decl(&mut self) -> Option<Stmt> {
        let stmt: anyhow::Result<Stmt> = if self.matches(&[Class]) {
            self.class_decl()
        } else if self.matches(&[Fun]) {
            self.fun_decl(FunctionKind::Function)
        } else if self.matches(&[Var]) {
            self.var_decl()
        } else {
            self.stmt()
        };

        if let Ok(stmt) = stmt {
            Some(stmt)
        } else {
            self.synchronize();
            None
        }
    }

    fn class_decl(&mut self) -> anyhow::Result<Stmt> {
        self.consume(Identifier, "Expect class name.")?;
        let name = self.previous.clone();

        self.consume(LeftBrace, "Expect '{' before class body.")?;
        let mut methods: Vec<Stmt> = vec![];
        while !self.check(&RightBrace) {
            methods.push(self.fun_decl(FunctionKind::Method)?);
        }
        self.consume(RightBrace, "Expect '}' after class body.")?;

        Ok(Stmt::Class(name, methods))
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
                self.consume(Identifier, "Expect parameter name.")?;

                if params.len() >= 255 {
                    let msg = "Can't have more than 255 parameters.";
                    self.error(msg);
                    anyhow::bail!(msg.to_string());
                }

                params.push(self.previous.clone());
            }
        }
        self.consume(RightParen, "Expect ')' after parameters.")?;

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
            if let Some(stmt) = self.decl() {
                statements.push(stmt);
            }
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
        self.consume(Semicolon, "Expect ';' after expression.")?;

        Ok(Stmt::Expression(expr))
    }

    fn error(&mut self, message: &str) {
        compile_time_error(Some(&self.previous), message);
        self.had_error = true;
    }

    fn handle_unterminated_string(&mut self, message: &str) {
        eprintln!("[line {}] Error: {message}", &self.previous.line);
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
            match l_expr {
                Expr::Variable(token) => {
                    return Ok(Expr::Assign(Assign {
                        name: token,
                        value: Box::new(self.assignment()?),
                    }));
                }
                Expr::Get(get) => {
                    return Ok(Expr::Set(Set {
                        expr: get.expr,
                        name: get.name,
                        value: Box::new(self.assignment()?),
                    }));
                }
                _ => {
                    let msg = "Invalid assignment target.";
                    self.error(msg);
                    anyhow::bail!(msg);
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
            } else if self.matches(&[Dot]) {
                self.consume(Identifier, "Expect property name after '.'.")?;
                let name = self.previous.clone();
                expr = Expr::Get(Get {
                    expr: Box::new(expr),
                    name,
                })
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
                args.push(self.expr()?);
                if args.len() > 255 {
                    let msg = "Can't have more than 255 arguments.";
                    self.error(msg);
                    anyhow::bail!(msg);
                }
            }
        }

        self.consume(RightParen, "Expect ')' after arguments.")?;

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
            anyhow::bail!(msg);
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
            UnterminatedString => {
                let msg = "Unterminated string.";
                self.handle_unterminated_string(msg);
                anyhow::bail!(msg);
            }
            _ => {
                let msg = "Expect expression.";
                self.error(msg);
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
            self.advance();
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
