use std::env;
use std::io::{self, BufRead, Write};
use std::fs;

mod lexer;
mod token;
mod parser;
mod ast;
mod error;
mod resolver;
mod checker;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 2 {
        println!("Usage: Rhype [script]");
        std::process::exit(64);
    }
    else if args.len() == 2 {
        run_file (&args[1])
    }
    else {
        run_prompt();
    }
} 

fn run(source: &str) {
    let mut lexer = lexer::Lexer::new(source.to_string());
    match lexer.scan_tokens() {
        Ok(tokens) => {
            let mut _parser = parser::Parser::new(tokens.clone());
            match _parser.parse() {
                Ok(stmts) => {
                    let mut _resolver = resolver::Resolver::new();
                    match _resolver.resolve(&stmts) {
                        Ok(()) => {
                            let mut _checker = checker::TypeChecker::new();
                            match _checker.check(&stmts) {
                                Ok(()) => println!("{:?}", stmts),
                                Err(e) => error::report_type(source, &e),
                            }
                        },
                        Err(e) => error::report_parse(source, &e),
                    }
                }
                Err(e) => error::report_parse(source, &e),
            }
            println!("{:?}", tokens);
        }
        Err(e) => error::report_lex(source, &e),
    }
}

fn run_file(path: &str) {
    let source = fs::read_to_string(path)
        .expect("Could not read file");
    run(&source);
}

fn run_prompt() {
    let stdin = io::stdin();
    io::stdout().flush().unwrap();

    loop {        
        println!("> ");

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).unwrap() == 0 {
            break;
        }
        run(line.trim());
    }
}