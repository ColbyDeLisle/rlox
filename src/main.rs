use rlox::Literal;
use rlox::expr::{Binary, Expr, Grouping, Unary};
use rlox::lexer::Lexer;
use rlox::parser::Parser;
use rlox::tokens::{Token, TokenType};

fn main() {
    // let source = r#"
    //     var x = 42;
    //
    //     var y = true;
    //     print "Hello,
    //     world!";
    //     var z = % false;
    // "#;

    let source = r#"
        6 * ((1 + 2) / (3 + (4)))
    "#;

    // let source = r#"
    //     1 + 2
    // "#;

    // let source = r#"
    // "#;

    let mut lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);

    let result = parser.unwrap().parse();

    match result {
        Ok(expr) => {
            println!("{}", expr);
        }
        _ => {
            println!("found error");
        }
    }

    // for token in lexer {
    //     println!("{:?}", token);
    // }
    //
    // let minus_123 = Expr::Unary(Unary {
    //     operator: Token {
    //         token_type: TokenType::Minus,
    //         lexeme: "-".to_string(),
    //         literal: None,
    //         line: 1,
    //     },
    //     right: Box::new(Expr::Literal(Literal::Number(123.0))),
    // });
    // let group_45_67 = Expr::Grouping(Grouping {
    //     expression: Box::new(Expr::Literal(Literal::Number(45.67)))
    // });
    // let product = Expr::Binary(Binary {
    //     left: Box::new(minus_123),
    //     operator: Token {
    //         token_type: TokenType::Star,
    //         lexeme: "*".to_string(),
    //         literal: None,
    //         line: 1,
    //     },
    //     right: Box::new(group_45_67),
    // });
    //
    // println!("{}", &product);
}
