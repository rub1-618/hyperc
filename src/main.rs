use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf};

use crate::error::{exit_code, report_all};

mod lexer;
mod token;
mod parser;
mod ast;
mod error;
mod resolver;
mod checker;
mod codegen;
mod support;

#[derive(Parser)]
#[command(version, about, name = "hyperc")]
struct Cli {
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Sets up the compiler and installs LLVM.
    Setup,

    /// Runs the file specified. If the file path is empty runs a terminal version.
    Run   {
        path: PathBuf,
        /// lets you select the directory for the binary file.
        #[arg(short, long, value_name = "DIRECTORY")]
        out: Option<PathBuf>,
        /// displays debug information
        #[arg(long)] debug: bool
    },

    /// Builds the file specified.
    Build {
        path: PathBuf,
        /// lets you select the directory for the binary file.
        #[arg(short, long, value_name = "DIRECTORY")]
        out: Option<PathBuf>,
        /// displays debug information
        #[arg(long)] debug: bool
    },
}

fn main() {
    let cli  = Cli::parse();
    match cli.command {
        Commands::Setup => {}
    
        Commands::Run { path, out, debug } => {
            build(path, out, debug);
            println!("Done!")

        }

        Commands::Build { path, out, debug } => {
            build(path, out, debug);
            println!("Done!")
        }
    }
}

fn build (path: PathBuf, out: Option<PathBuf>, debug: bool) {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            panic!("Unable to read the file: {e}")
        }
    };

    let mut lexer = lexer::Lexer::new(source.clone());
    let tokens = lexer.scan_tokens();
    if !lexer.errors.is_empty() {
        report_all(&source, &lexer.errors);
        std::process::exit(exit_code(&lexer.errors[0]))
    }

    let mut parser = parser::Parser::new(tokens);
    let stmts = parser.parse();
    if !parser.errors.is_empty() {
        report_all(&source, &parser.errors);
        std::process::exit(exit_code(&parser.errors[0]))
    }

    let mut resolver = resolver::Resolver::new();
    resolver.resolve(&stmts);
    if !resolver.errors.is_empty() {
        report_all(&source, &resolver.errors);
        std::process::exit(exit_code(&resolver.errors[0]))
    }
    
    let types = resolver.get_types();
    let mut checker = checker::TypeChecker::new(types);
    checker.check(&stmts);
    if !checker.errors.is_empty() {
        report_all(&source, &checker.errors);
        std::process::exit(exit_code(&checker.errors[0]))
    }


    println!("{:?}", stmts);
    println!("out: {:?}", out);
    println!("debug: {:?}", debug);

    std::process::exit(0);
}

// fn run(out: Option<PathBuf>) {
//     let out = match out {
//         Some(o) => o.to_str().unwrap().to_string(),
//         None => "./out".to_string()
//     };
//     let run = std::process::Command::new(out);

// }