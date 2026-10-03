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

    /// Runs the file specified. If the file path is empty runs a terminal version.
    Run   {
        path: PathBuf,
        /// lets you select the directory for the binary file.
        #[arg(short, long, value_name = "DIRECTORY")]
        out: Option<PathBuf>,

        /// displays lexer tokens
        #[arg(long)] show_tokens: bool,

        /// displays the checked ast if it has no mistakes
        #[arg(long)] show_ast: bool,

        /// displays the generated intermediate representation
        #[arg(long)] show_ir: bool
    },

    /// Builds the file specified.
    Build {
        path: PathBuf,
        /// lets you select the directory for the binary file.
        #[arg(short, long, value_name = "DIRECTORY")]
        out: Option<PathBuf>,

        /// displays lexer tokens
        #[arg(long)] show_tokens: bool,

        /// displays the checked ast if it has no mistakes
        #[arg(long)] show_ast: bool,

        /// displays the generated intermediate representation
        #[arg(long)] show_ir: bool
    },
}

fn main() {
    let cli  = Cli::parse();
    match cli.command {
    
        Commands::Setup => setup::setup_hyperc(),

        Commands::Run { path, out, 
            show_tokens, show_ast, show_ir } => {
            run(&path, &out, show_tokens, show_ast, show_ir);

        }

        Commands::Build { path, out, 
            show_tokens, show_ast, show_ir } => {
            build(&path, &out, show_tokens, show_ast, show_ir);
        }
    }
}

fn build (path: &PathBuf, out: &Option<PathBuf>, 
    show_tokens: bool, show_ast: bool, show_ir: bool) -> PathBuf {

    let src = drivers::get_source(&path);
    let path_str = drivers::get_path_str(&path);
    let out_str = drivers::get_out_str(&out);

    // ! lexer
    let tokens = drivers::get_tokens(&src);

    if show_tokens { println!("{:?}", tokens); }

    // ! parser
    let stmts = drivers::get_stmts(&src, tokens);

    // ! resolver + checker
    drivers::check_stmts(&src, &stmts);

    if show_ast { println!("{:?}", stmts); }

    // ! codegen
    let bin_path = drivers::generate_code(
        &src, &stmts, &path_str, &out_str, show_ir
    );
    
    println!("Compiled to: {}\n", &bin_path.display());
    bin_path
}

fn run (path: &PathBuf, out: &Option<PathBuf>, 
    show_tokens: bool, show_ast: bool, show_ir: bool) {
    
    let bin_path = build(path, out, show_tokens, show_ast, show_ir);
    drivers::run_bin(&bin_path);
}