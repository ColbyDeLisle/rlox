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
    let mut interpreter = Interpreter::new();

    match run(source.as_str(), &mut interpreter) {
        Ok(_) => {}
        Err(ErrorKind::Compile) => process::exit(65),
        Err(ErrorKind::Runtime) => process::exit(70),
    }
}

fn run_repl() {
    let stdin = io::stdin();
    let mut line = String::new();
    let mut interpreter = Interpreter::new();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        line.clear();
        match stdin.read_line(&mut line) {
            Ok(0) | Err(_) => break, // EOF (Ctrl+D on Unix, Ctrl+Z on Windows)
            Ok(_) => {}
        }

        if let Err(err) = run(line.as_str(), &mut interpreter) {
            match err {
                ErrorKind::Compile => eprintln!("Compile error."),
                ErrorKind::Runtime => eprintln!("Runtime error."),
            }
        }
    }
}

fn run(source: &str, interpreter: &mut Interpreter) -> Result<(), ErrorKind> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer);

    let Ok(stmts) = parser.parse() else {
        return Err(ErrorKind::Compile);
    };

    match interpreter.interpret(stmts) {
        Ok(_) => Ok(()),
        Err(Signal::ResolveError) => Err(ErrorKind::Compile),
        Err(_) => Err(ErrorKind::Runtime),
    }
}
