use std::{fs, path::PathBuf, process::Command};
use inkwell;

use crate::token;
use crate::ast;
use crate::lexer;
use crate::parser;
use crate::resolver;
use crate::checker;
use crate::codegen;
use crate::error;

pub fn run_bin(bin_path: &PathBuf) {
    let mut cmd = Command::new(bin_path);
    match cmd.status() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error ocured on runtime: {}", e)
        }
    }
}

pub fn get_out_str(out: &Option<PathBuf>) -> String {
    match out {
        Some(o) => o.to_str().unwrap().to_string(),
        None => "out".to_string()
    }
}

pub fn get_path_str(path: &PathBuf) -> String {
    if !path.exists() {
        panic!("File not found.")
    }
    check_extension(path);
    path.to_str().unwrap().to_string()
}

fn check_extension(path: &PathBuf) {
    let ext = "hr".to_string();
    match path.extension() {
        Some(extension) => {
            if extension.to_str().unwrap().to_string() != ext {
                panic!("Not a '.hr' file.")
            }
        }
        None => {
            panic!("This file has no extension.")
        }
    }
}

pub fn get_source(path: &PathBuf) -> String {
    match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            panic!("Unable to read the file: {e}")
        }
    }
}

pub fn get_tokens(src: &str) -> Vec<token::Token> {
    let mut lexer = lexer::Lexer::new(src.to_string());
    let tokens = lexer.scan_tokens();
    if !lexer.errors.is_empty() {
        error::report_all(&src, &lexer.errors);
        std::process::exit(error::exit_code(&lexer.errors[0]))
    }
    tokens
}

pub fn get_stmts(src: &str, tokens: Vec<token::Token>) -> Vec<ast::Stmt> {
    let mut parser = parser::Parser::new(tokens);
    let stmts = parser.parse();
    if !parser.errors.is_empty() {
        error::report_all(&src, &parser.errors);
        std::process::exit(error::exit_code(&parser.errors[0]))
    }
    stmts
}

pub fn check_stmts(src: &str, stmts: &Vec<ast::Stmt>) {

    let mut resolver = resolver::Resolver::new();
    resolver.resolve(&stmts);
    if !resolver.errors.is_empty() {
        error::report_all(&src, &resolver.errors);
        std::process::exit(error::exit_code(&resolver.errors[0]))
    }
    
    let types = resolver.get_types();
    let mut checker = checker::TypeChecker::new(types);
    checker.check(&stmts);
    if !checker.errors.is_empty() {
        error::report_all(&src, &checker.errors);
        std::process::exit(error::exit_code(&checker.errors[0]))
    }
}

pub fn generate_code(src: &str, stmts: &Vec<ast::Stmt>, 
 path: &str, out: &str, is_debug: bool) -> PathBuf {
    let context = inkwell::context::Context::create();
    let mut codegen = codegen::Codegen::new(&context);
    let bin_path = match codegen.compile(&stmts, path, out, is_debug) {
        Ok(p) => p,
        Err(e) => {
            error::report_error(&src, &e);
            std::process::exit(error::exit_code(&e))
        }
    };
    bin_path
}