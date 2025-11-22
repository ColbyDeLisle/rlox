use rlox::interpreter::Interpreter;
use rlox::lexer::Lexer;
use rlox::parser::Parser;

fn main() -> anyhow::Result<()> {
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
    //     (1 + 2) == ((2 * 2) + 5) / 3.0
    // "#;

    // let source = r#"
    // "#;

    // let source = r#"
    // 2 * false
    // "#;

    // let source = r#"
    //     print "one";
    //     print !true;
    //     print 2 + 1;
    //
    //     var a = 1;
    //     var b = 12;
    //     print a + b;
    //
    //     var c;
    //     print c = "hello";
    // "#;

    // let source = r#"
    //     var a = "global a";
    //     var b = "global b";
    //     var c = "global c";
    //     {
    //       var a = "outer a";
    //       var b = "outer b";
    //       {
    //         var a = "inner a";
    //         print a;
    //         print b;
    //         print c;
    //       }
    //
    //       {
    //         var a = "other inner a";
    //         print a;
    //       }
    //
    //       print a;
    //       print b;
    //       print c;
    //     }
    //     print a;
    //     print b;
    //     print c;
    //
    //     print "hi" or 2;
    //     print nil or "yes";
    //     print nil and "maybe";
    //     print "possibly" and "maybe";
    //
    //     var i = 0;
    //     while (i < 10) {
    //         print i;
    //         i = i + 1;
    //     }
    //
    // "#;

    let source = r#"
        var a = 0;
        var temp;

        for (var b = 1; a < 10000; b = temp + b) {
          print a;
          temp = a;
          a = b;
        }
    "#;

    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer).unwrap();
    let mut interpreter = Interpreter::new();
    let stmts = parser.parse().unwrap();

    interpreter.interpret(stmts)
}
