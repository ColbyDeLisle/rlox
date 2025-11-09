use rlox::lexer::Lexer;

fn main() {
    let source = r#"
        var x = 42;

        var y = true;
        print "Hello,
        world!";
        var z = % false;
    "#;

    let mut lexer = Lexer::new(source);

    while let Some(token) = lexer.next_token() {
        println!("{:?}", token);
    }
}
