use std::env::{self, args};
use std::io::{self, BufRead, Write};
use std::fs;

use inkwell::context::Context;
use codegen::Codegen;

mod lexer;
mod token;
mod parser;
mod ast;
mod error;
mod resolver;
mod checker;
mod codegen;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        run_prompt();
    } else if args.len() == 2 {
        if &args[1] == "--help" || &args[1] == "-h" {
            println!("Usage: hyperc <script> [-o name] | -h | -V");
            std::process::exit(0);
        } else if &args[1] == "--version" || &args[1] == "-V" {
            let version = env!("CARGO_PKG_VERSION");
            println!("hyperc: {}", version);
            std::process::exit(0);
        } else {
            run_file(&args[1], "out");
        }
    } else if args.len() == 4 {
        if &args[2] == "-o" {
            let out = &args[3];
            run_file(&args[1], out);
        } else {
            println!("Unknown argument.");
            std::process::exit(64);
        }
    } else {
        println!("Unknown argument.");
        std::process::exit(64);
    }
}

fn run(source: &str, path: &str, out_name: &str) {
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
                                Ok(()) => {
                                    // println!("{:?}", &stmts);
                                    let context = Context::create();
                                    let mut codegen = Codegen::new(&context);
                                    match codegen.compile(&stmts, path, out_name) {
                                        Ok(()) => {},
                                        Err(e) => {
                                            error::report_compile(source, &e);
                                            std::process::exit(69);
                                        },
                                    }
                                },
                                Err(e) => {
                                    error::report_type(source, &e);
                                    std::process::exit(68);
                                },
                                
                            }
                        },
                        Err(e) => {
                            error::report_parse(source, &e);
                            std::process::exit(67);
                        },
                    
                    }
                }
                Err(e) => {
                    error::report_parse(source, &e);
                    std::process::exit(66);
                },
            }
            // println!("{:?}", tokens);
        }
        Err(e) => {
            error::report_lex(source, &e);
            std::process::exit(65)
        },
    }
}

fn run_file(path: &str, out_name: &str) {
    let source = fs::read_to_string(path)
        .expect("Could not read file");
    run(&source, path, out_name);
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
        run(line.trim(), "./", "out");
    }
}