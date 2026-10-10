use crate::{
    codegen::Emit, drivers::{StageKind, log_stage}, error::{ColorMode, exit_code, report_single_error}
};
use clap::{Parser, Subcommand};
use std::{path::PathBuf, time::Instant};

mod lexer;
mod token;
mod parser;
mod ast;
mod error;
mod resolver;
mod checker;
mod codegen;
mod support;
mod drivers;
mod setup;

const RUN_EMIT: Emit = Emit::Obj;

#[derive(Parser)]
#[command(version, about, name = "hyperc")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {

    /// Sets up the compiler and installs llvm.
    Setup,

    /// Runs the file specified.
    Run   {
        path: PathBuf,
        /// lets you select the directory for the binary file.
        #[arg(short, long, value_name = "DIRECTORY")]
        out: Option<PathBuf>,

        /// displays lexer tokens.
        #[arg(long)] show_tokens: bool,

        /// displays the checked ast if it has no mistakes.
        #[arg(long)] show_ast: bool,

        /// displays the generated intermediate representation.
        #[arg(long)] show_ir: bool,

        /// sets the color stderr mode ( auto / always / never ).
        #[arg(long, value_enum, default_value_t = ColorMode::Auto)] color: ColorMode,

        /// gives a more detailed information about the compilers work.
        #[arg(long, short)] verbose: bool
    },

    /// builds the file specified.
    Build {
        path: PathBuf,
        /// lets you select the directory for the binary file.
        #[arg(short, long, value_name = "DIRECTORY")]
        out: Option<PathBuf>,

        /// displays lexer tokens.
        #[arg(long)] show_tokens: bool,

        /// displays the checked ast if it has no mistakes.
        #[arg(long)] show_ast: bool,

        /// displays the generated intermediate representation.
        #[arg(long)] show_ir: bool,

        /// sets the color stderr mode ( auto / always / never ).
        #[arg(long, value_enum, default_value_t = ColorMode::Auto)] color: ColorMode,

        /// lets you generate a linked file ( obj / asm ).
        #[arg(long, value_enum, default_value_t = Emit::Obj)] emit: Emit,

        /// gives a more detailed information about the compilers work.
        #[arg(long, short)] verbose: bool
    },

    /// checks the file specified.
    Check {
        path: PathBuf,

        /// displays lexer tokens.
        #[arg(long)] show_tokens: bool,

        /// displays the checked ast if it has no mistakes.
        #[arg(long)] show_ast: bool,

        /// sets the color stderr mode ( auto / always / never ).
        #[arg(long, value_enum, default_value_t = ColorMode::Auto)] color: ColorMode,

        /// gives a more detailed information about the compilers work.
        #[arg(long, short)] verbose: bool
    }
}

fn main() {
    let cli  = Cli::parse();
    match cli.command {
    
        Commands::Setup => setup::setup_hyperc(),

        Commands::Run { 
            path, out, 
            show_tokens, show_ast, 
            show_ir, color, 
            verbose 
        } => {
            let before = Instant::now();
            let code = run(&path, &out, show_tokens, show_ast, show_ir, color, verbose);
            log_stage(StageKind::All, before, verbose);
            std::process::exit(code);
        }

        Commands::Build {
            path, out, 
            show_tokens, show_ast,
            show_ir, color,
            emit, verbose
        } => {
            let before = Instant::now();
            let bin_path = build(
                &path, &out, 
                show_tokens, show_ast, 
                show_ir, 
                color, emit.clone(),
                verbose
            );
            log_stage(StageKind::Compiling, before, verbose);

            if emit != Emit::Ll {
                eprintln!("Compiled to: {}\n", &bin_path.display());
            }
                
        }

        Commands::Check { 
            path, show_tokens,
            show_ast, color, 
            verbose
        } => {
            let before = Instant::now();
            let is_color = error::is_color(color);
            check(&path, show_tokens, show_ast, is_color, verbose);
            log_stage(StageKind::All, before, verbose);
            eprintln!("ok");
        }
    }
}

fn build (path: &PathBuf, out: &Option<PathBuf>, 
    show_tokens: bool, show_ast: bool, show_ir: bool,
    colormode: ColorMode, emit: Emit, verbose: bool
) -> PathBuf {

    let path_str = match drivers::get_path_str(&path) {
        Ok(p) => p,
        Err(e) => {
            error::report_single_error(&e); // single FileError only
            std::process::exit(exit_code(&e))
        }
    };
    let out_str = drivers::get_out_str(&out);
    let  is_color = error::is_color(colormode);
    
    let (stmts, src) = check(&path, show_tokens, show_ast, is_color, verbose);

    // ! codegen
    let before = Instant::now();
    let bin_path = drivers::generate_code(
        &src, &stmts, &path_str, &out_str, show_ir, is_color, emit
    );
    log_stage(StageKind::Codegen, before, verbose);

    bin_path
}

fn run (
    path: &PathBuf, out: &Option<PathBuf>, 
    show_tokens: bool, show_ast: bool, 
    show_ir: bool, colormode: ColorMode, 
    verbose: bool
) -> i32 {
    let before = Instant::now();
    let bin_path = build(
        path, out, 
        show_tokens, show_ast, 
        show_ir, colormode, 
        RUN_EMIT, verbose
    );
    log_stage(StageKind::Compiling, before, verbose);

    // Emit::Obj because it's the only option for run
    let before = Instant::now();
    let code = drivers::run_bin(&bin_path);
    match code {
        Ok(opt) => {
            match opt {
                Some(c) => {
                    log_stage(StageKind::Run, before, verbose);
                    c
                }
                None => {
                    let error = error::HypercError::RunTimeError {
                        message: "Unable to determine the exit code.".to_string()
                    };
                    error::report_single_error(&error);
                    error::exit_code(&error)
                }
            }
        }
        Err(error) => {
            error::report_single_error(&error); 
            error::exit_code(&error)
        }
    }
}

fn check(path: &PathBuf, show_tokens: bool, show_ast: bool, is_color: bool, verbose: bool
) -> (Vec<ast::Stmt>, String) {
    let src = match drivers::get_source(&path) {
        Ok(s) => s,
        Err(e) => {
            report_single_error(&e); // FileError
            std::process::exit(exit_code(&e));
        }
    };

    // ! lexer
    let before = Instant::now();
    let tokens = drivers::get_tokens(&src,  is_color);
    log_stage(StageKind::Lexer, before, verbose);

    if show_tokens { println!("{:?}", tokens); }

    // ! parser
    let before = Instant::now();
    let stmts = drivers::get_stmts(&src, tokens, is_color);
    log_stage(StageKind::Parser, before, verbose);

    // ! resolver + checker
    let before = Instant::now();
    drivers::check_stmts(&src, &stmts, is_color);
    log_stage(StageKind::Checking, before, verbose);

    if show_ast { println!("{:?}", stmts); }

    (stmts, src)
            
}