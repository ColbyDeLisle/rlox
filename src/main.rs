use rlox::interpreter::Interpreter;
use rlox::lexer::Lexer;
use rlox::parser::Parser;

fn main() {
    // Some test cases:

    // let source = r#"
    //     var x = 42;
    //
    //     var y = true;
    //     print "Hello,
    //     world!";
    //     var z = % false;
    // "#;

    // let source = r#"
    //     6 * ((1 + 2) / (3 + (4)))
    // "#;

    // let source = r#"
    //     1 + 2
    // "#;

    // let source = r#"
    //     (1 + 2) == ((2 * true) + 5) / 3.0
    // "#;

    // let source = r#"
    // "#;

    let source = r#"
    2 * false
    "#;

    let lexer = Lexer::new(source);
    let parser = Parser::new(lexer);
    let mut interpreter = Interpreter::new();

    let expr = parser.unwrap().parse().unwrap();
    let value = interpreter.expr(expr);

    match value {
        Ok(val) => {
            println!("{}", val);
        }
        _ => {
            println!("found error");
        }
    }
}
