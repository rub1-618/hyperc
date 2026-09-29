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
    
        Commands::Run { path, out, debug } => {
            run(&path, &out, debug);

        }

        Commands::Build { path, out, debug } => {
            build(&path, &out, debug);
        }
    }
}

fn build (path: &PathBuf, out: &Option<PathBuf>, debug: bool) -> PathBuf {
    let src = drivers::get_source(&path);
    let path_str = drivers::get_path_str(&path);
    let out_str = drivers::get_out_str(&out);

    // lexer
    let tokens = drivers::get_tokens(&src);
    // parser
    let stmts = drivers::get_stmts(&src, tokens);
    // resolver + checker
    drivers::check_stmts(&src, &stmts);
    // codegen
    let bin_path = drivers::generate_code(
        &src, &stmts, &path_str, &out_str, debug
    );
    
    println!("Compiled to: {}\n", &bin_path.display());
    bin_path
}

fn run (path: &PathBuf, out: &Option<PathBuf>, debug: bool) {
    let bin_path = build(path, out, debug);
    drivers::run_bin(&bin_path);
}