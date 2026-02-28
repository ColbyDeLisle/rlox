use rlox::interpreter::Interpreter;
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
    let _program = args.next();

    match (args.next(), args.next()) {
        (Some(path), None) => {
            run_file(&path.as_str());
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

    match run(&source.as_str()) {
        Ok(_) => {}
        Err(ErrorKind::Compile) => process::exit(65),
        Err(ErrorKind::Runtime) => process::exit(70),
    }
}

fn run(source: &str) -> Result<(), ErrorKind> {
    let lexer = Lexer::new(source);

    let Some(mut parser) = Parser::new(lexer) else {
        return Err(ErrorKind::Compile);
    };

    let Ok(stmts) = parser.parse() else {
        return Err(ErrorKind::Compile);
    };

    match Interpreter::new().interpret(stmts) {
        Ok(_) => Ok(()),
        Err(_) => Err(ErrorKind::Runtime),
    }
}
