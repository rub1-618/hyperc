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
mod matching;

#[derive(Parser)]
#[command(version, name = "hyperc")]
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
        path: Option<PathBuf>,
        /// lets you select the directory for the binary file.
        #[arg(short, long, value_name = "DIRECTORY")]
        out: Option<PathBuf>,
        /// displays debug information
        #[arg(long)] debug: bool
    },

    /// Builds the file specified.
    Build {
        path: Option<PathBuf>,
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

        }

        Commands::Build { path, out, debug } => {}

    }
}

fn build () {

}