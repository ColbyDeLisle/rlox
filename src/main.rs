use rlox::interpreter::{Interpreter, Signal};
use rlox::lexer::Lexer;
use rlox::parser::Parser;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

#[derive(Debug)]
enum ErrorKind {
    Compile,
    Runtime,
}
fn main() {
    let mut args = env::args();
    let _program = args.next(); // skip program name

    match (args.next(), args.next()) {
        (None, _) => {
            // No arguments → REPL
            run_repl();
        }
        (Some(path), None) => {
            // One argument → run file
            run_file(path.as_str());
        }
        _ => {
            eprintln!("Usage: rlox [script]");
            process::exit(64);
        }
    }
}

fn run_file(path: &str) {
    let source = fs::read_to_string(path).unwrap_or_else(|err| {
        eprintln!("Could not read file: {err}");
        process::exit(74);
    });

    match run(source.as_str()) {
        Ok(_) => {}
        Err(ErrorKind::Compile) => process::exit(65),
        Err(ErrorKind::Runtime) => process::exit(70),
    }
}

fn run_repl() {
    let stdin = io::stdin();
    let mut line = String::new();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        line.clear();
        if stdin.read_line(&mut line).unwrap() == 0 {
            break; // EOF (Ctrl+D)
        }

        if let Err(err) = run(line.as_str()) {
            match err {
                ErrorKind::Compile => eprintln!("Compile error."),
                ErrorKind::Runtime => eprintln!("Runtime error."),
            }
        }
    }
}

fn run(source: &str) -> Result<(), ErrorKind> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);

    let Ok(stmts) = parser.parse() else {
        return Err(ErrorKind::Compile);
    };

    match Interpreter::new().interpret(stmts) {
        Ok(_) => Ok(()),
        Err(Signal::ResolveError) => Err(ErrorKind::Compile),
        Err(_) => Err(ErrorKind::Runtime),
    }
}

// Test using the following:
// (replace "chap04_scanning" with "jlox" or "clox" or any other specific chapter)
// dart run tool/bin/test.dart chap04_scanning --interpreter ~/Documents/rlox/target/release/rlox

// TEST TODOs:
// * address float parsing without preceding digit (e.g. '.123')
// * Print expr stmts; per chatGPT:
// Inside your expression-statement execution:
//
// fn execute_expression_stmt(&self, expr: &Expr) -> Result<(), LoxError> {
//     let value = self.evaluate(expr)?;
//     println!("{}", value.to_test_string());
//     Ok(())
// }
//
// The test suite expects all evaluated top-level expressions to print in this exact format.
//
// Statements like print or var already handle their own output.
//
// But bare literals (like 123) need this explicit printing.
