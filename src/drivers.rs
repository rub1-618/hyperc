use std::{fs, path::PathBuf, process::Command, time::Instant};
use inkwell;

use crate::codegen::Emit;
use crate::error::HypercError;
use crate::token;
use crate::ast;
use crate::lexer;
use crate::parser;
use crate::resolver;
use crate::checker;
use crate::codegen;
use crate::error;

pub enum StageKind {
    Lexer,
    Parser,
    Checking,
    Codegen,
    
    Compiling,
    Run,
    All
}

pub fn run_bin(bin_path: &PathBuf) -> Result<Option<i32>, HypercError> {
    let mut cmd = Command::new(bin_path);
    match cmd.status() {
        Ok(status) => Ok(status.code()),
        Err(e) => Err(HypercError::RunTimeError {
            message: format!("Error ocured on runtime: {}", e)
        })
    }
}

pub fn get_out_str(out: &Option<PathBuf>) -> String {
    match out {
        Some(o) => o.to_str().unwrap().to_string(),
        None => "out".to_string()
    }
}

pub fn log_stage(stage: StageKind, before: Instant, verbose: bool) {
    if verbose {
        let stage = match stage {
            StageKind::Lexer        => "token scanning",
            StageKind::Parser       => "AST building",
            StageKind::Checking     => "AST checking",
            StageKind::Codegen      => "code generation",
            StageKind::Compiling    => "compiling process",
            StageKind::Run          => "running process",
            StageKind::All          => "all"
        };
        let time = before.elapsed();
        let res = format!("Finished {stage} in: {time:.2?}.");
        eprintln!("{}", res);
    }
}

pub fn get_path_str(path: &PathBuf) -> Result<String, HypercError> {
    if !path.exists() {
        return Err(HypercError::FileError {
            message: format!("File '{:?}' not found.", path.display())
        });
    }
    check_extension(path)?; // FileErrors only (so that it can be 'report_single_error')
    Ok(path.to_str().unwrap().to_string())
}

fn check_extension(path: &PathBuf) -> Result<(), HypercError> {
    let ext = "hr".to_string();
    match path.extension() {
        Some(extension) => {
            if extension.to_str().unwrap().to_string() != ext {
                Err(HypercError::FileError {
                    message: "Not a '.hr' file.".to_string()
                })
            } else { Ok(()) }
        }
        None => {
            Err(HypercError::FileError {
                message: "This file has no extension.".to_string()
            })
        }
    }
}

pub fn get_source(path: &PathBuf) -> Result<String, HypercError> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(s),
        Err(e) => {
            Err(HypercError::FileError {
                message: format!("Unable to read the file: {e}")
            })
        }
    }
}

pub fn get_tokens(src: &str, is_color: bool) -> Vec<token::Token> {
    let mut lexer = lexer::Lexer::new(src.to_string());
    let tokens = lexer.scan_tokens();
    if !lexer.errors.is_empty() {
        error::report_compiler_all(&src, &lexer.errors, is_color);
    }
    tokens
}

pub fn get_stmts(src: &str, tokens: Vec<token::Token>, is_color: bool) -> Vec<ast::Stmt> {
    let mut parser = parser::Parser::new(tokens);
    let stmts = parser.parse();
    if !parser.errors.is_empty() {
        error::report_compiler_all(&src, &parser.errors, is_color);
    }
    stmts
}

pub fn check_stmts(src: &str, stmts: &Vec<ast::Stmt>, is_color: bool) {

    let mut resolver = resolver::Resolver::new();
    resolver.resolve(&stmts);
    if !resolver.errors.is_empty() {
        error::report_compiler_all(&src, &resolver.errors, is_color);
    }
    
    let types = resolver.get_types();
    let mut checker = checker::TypeChecker::new(types);
    checker.check(&stmts);
    if !checker.errors.is_empty() {
        error::report_compiler_all(&src, &checker.errors, is_color);
    }
}

pub fn generate_code(
    src: &str, stmts: &Vec<ast::Stmt>, 
    path: &str, out: &str, show_ir: bool, 
    is_color: bool, emit: Emit
) -> PathBuf {

    let context = inkwell::context::Context::create();
    let mut codegen = codegen::Codegen::new(&context);
    let bin_path = match codegen.compile(&stmts, path, out, show_ir, emit) {
        Ok(p) => p,
        Err(e) => {
            match e {
                error::HypercError::CompileError { .. } => {
                    error::report_compiler_error(src, &e, is_color);
                    std::process::exit(error::exit_code(&e));
                }

                error::HypercError::BuildError { .. } => {
                    error::report_single_error(&e);
                    std::process::exit(error::exit_code(&e));
                }

                _ => unreachable!() // bcs only Compile- and
                // Build- Errors are handled in codegen.rs
            }
        }
    };
    bin_path
}