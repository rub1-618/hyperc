use ariadne::{ Label, Report, ReportKind, Source };
use std::{io::IsTerminal, ops::Range};

#[derive(Debug, Clone)]
pub enum HypercError {
    LexerError      { span: Range<usize>, message: String },
    ParseError      { span: Range<usize>, message: String },
    ResolveError    { span: Range<usize>, message: String },
    TypeError       { span: Range<usize>, message: String },
    CompileError    { span: Range<usize>, message: String },
    BuildError      { message: String },
    RunTimeError    { message: String },
    FileError       { message: String },

    SetupError      { message: String },
}

pub fn report_compiler_error(source: &str, error: &HypercError, color: bool) {
    match &error {
        HypercError::LexerError { span, message }   |
        HypercError::ParseError { span, message }   |
        HypercError::ResolveError { span, message } |
        HypercError::TypeError { span, message }    |
        HypercError::CompileError { span, message } => {
            proceed_error(source, span, message, color);
        }
        _ => panic!("Not a compiler error.")
    }
}

pub fn report_single_error(error: &HypercError) {
    match &error {
        HypercError::BuildError { message } =>  {
            eprintln!("[BuildError] {message}");
            std::process::exit(exit_code(error));
        }
        HypercError::SetupError { message } =>  {
            eprintln!("[SetupError] {message}");
            std::process::exit(exit_code(error));
        }
        HypercError::RunTimeError { message } =>  {
            eprintln!("[RunTimeError] {message}");
        }
        HypercError::FileError { message } =>  {
            eprintln!("[FileError] {message}");
        }
        _ => panic!("Not a setup error.")
    }
}

fn proceed_error(
    source: &str, span: &Range<usize>, 
    message: &str, color: bool
) {
    // eprintln!("proceed_error color = {}", color);
    match color {
        true => {
            let label_color = ariadne::Color::Fixed(124);
            Report::build(ReportKind::Error, ("input", span.clone()))
                .with_message(message)
                .with_label(
                    Label::new(("input", span.clone()))
                    .with_message(message)
                    .with_color(label_color)
                )
                .finish()
                .eprint(("input", Source::from(source)))
                .unwrap()
        }
        false => {
            let config = ariadne::Config::new();
            let config = config.with_color(color);
            Report::build(ReportKind::Error, ("input", span.clone()))
                .with_message(message)
                .with_label(
                    Label::new(("input", span.clone()))
                    .with_message(message)
                ).with_config(config)
                .finish()
                .eprint(("input", Source::from(source)))
                .unwrap()
        }
    }
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ColorMode {
    Auto,
    Always,
    Never
}

pub fn is_color(colormode: ColorMode) -> bool {
    match colormode {
        ColorMode::Auto => {
            let stderr = std::io::stderr();
            if stderr.is_terminal() {
                true
            } else { false }
        }
        ColorMode::Always => true,
        ColorMode::Never => false
    }
}

pub fn report_compiler_all(source: &str, errors: &[HypercError], color: bool) {
    if errors.is_empty() { return; }
    for error in errors {
        report_compiler_error(source, error, color);
    }
    std::process::exit(exit_code(&errors[0]))
}

pub fn exit_code(error: &HypercError) -> i32 {
    match error {
        HypercError::LexerError     { .. } => 16,
        HypercError::ParseError     { .. } => 17,
        HypercError::ResolveError   { .. } => 18,
        HypercError::TypeError      { .. } => 19,
        HypercError::CompileError   { .. } => 20,
        HypercError::BuildError     { .. } => 21,
        HypercError::RunTimeError   { .. } => 22,
        HypercError::FileError      { .. } => 23,
        HypercError::SetupError     { .. } => 24
    }
}