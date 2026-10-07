use crate::{error::ColorMode, codegen::Emit};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    #[arg(short, long)]
    verbose: bool,

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
        #[arg(long, value_enum, default_value_t = ColorMode::Auto)] color: ColorMode
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
        #[arg(long, value_enum, default_value_t = Emit::Obj)] emit: Emit
    },

    /// checks the file specified.
    Check {
        path: PathBuf,

        /// displays lexer tokens.
        #[arg(long)] show_tokens: bool,

        /// displays the checked ast if it has no mistakes.
        #[arg(long)] show_ast: bool,

        /// sets the color stderr mode ( auto / always / never ).
        #[arg(long, value_enum, default_value_t = ColorMode::Auto)] color: ColorMode

    }
}

fn main() {
    let cli  = Cli::parse();
    match cli.command {
    
        Commands::Setup => setup::setup_hyperc(),

        Commands::Run { path, out, 
            show_tokens, show_ast, show_ir, color } => {
            
            run(&path, &out, show_tokens, show_ast, show_ir, color);
        }

        Commands::Build {
            path, out, 
            show_tokens, show_ast,
            show_ir, color,
            emit } => {
            
            let bin_path = build(&path, &out, show_tokens, show_ast, show_ir, color, emit.clone());
            if emit != Emit::Ll {
                eprintln!("Compiled to: {}\n", &bin_path.display());
            }
        }

        Commands::Check { path, 
            show_tokens, show_ast, color } => {
            let is_color = error::is_color(color);
            check(&path, show_tokens, show_ast, is_color);
            println!("ok")
        }
    }
}

fn build (path: &PathBuf, out: &Option<PathBuf>, 
    show_tokens: bool, show_ast: bool, show_ir: bool,
    colormode: ColorMode, emit: Emit
) -> PathBuf {

    let path_str = drivers::get_path_str(&path);
    let out_str = drivers::get_out_str(&out);
    let  is_color = error::is_color(colormode);
    
    let (stmts, src) = check(&path, show_tokens, show_ast, is_color);

    // ! codegen
    let bin_path = drivers::generate_code(
        &src, &stmts, &path_str, &out_str, show_ir, is_color, emit
    );

    bin_path
}

fn run (
    path: &PathBuf, out: &Option<PathBuf>, 
    show_tokens: bool, show_ast: bool, 
    show_ir: bool, colormode: ColorMode
) {
    
    let bin_path = build(path, out, show_tokens, show_ast, show_ir, colormode, RUN_EMIT);
    // Emit::Obj because it's the only option for run
    drivers::run_bin(&bin_path);
}

fn check(path: &PathBuf, show_tokens: bool, show_ast: bool, is_color: bool
) -> (Vec<ast::Stmt>, String) {

    let src = drivers::get_source(&path);

    // ! lexer
    let tokens = drivers::get_tokens(&src,  is_color);

    if show_tokens { println!("{:?}", tokens); }

    // ! parser
    let stmts = drivers::get_stmts(&src, tokens, is_color);

    // ! resolver + checker
    drivers::check_stmts(&src, &stmts, is_color);

    if show_ast { println!("{:?}", stmts); }

    (stmts, src)
}