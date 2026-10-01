use ariadne::{ Report, ReportKind, Label, Source };
use std::ops::Range;

#[derive(Debug, Clone)]
pub enum HypercError {
    LexerError      { span: Range<usize>, message: String },
    ParseError      { span: Range<usize>, message: String },
    ResolveError    { span: Range<usize>, message: String },
    TypeError       { span: Range<usize>, message: String },
    CompileError    { span: Range<usize>, message: String },
    BuildError      { message: String },

    SetupError      { message: String },
}

pub fn report_compiler_error(source: &str, error: &HypercError) {
    match &error {
        HypercError::LexerError { span, message }   |
        HypercError::ParseError { span, message }   |
        HypercError::ResolveError { span, message } |
        HypercError::TypeError { span, message }    |
        HypercError::CompileError { span, message } => {
            proceed_error(source, span, message);
        }
        HypercError::BuildError { message, .. } => {
            eprintln!("[BuildError] {message}")
        }
        _ => panic!("Not a compiler error.")
    }
}

pub fn report_setup_error(error: &HypercError) {
    match &error {
        HypercError::SetupError { message, .. } =>  {
            eprintln!("[SetupError] {message}");
            std::process::exit(exit_code(error));
        }
        _ => panic!("Not a setup error.")
    }
}

fn proceed_error(source: &str, span: &Range<usize>, message: &str) {
    let label_color = ariadne::Color::Fixed(124);
    Report::build(ReportKind::Error, ("input", span.clone()))
        .with_message(&message)
        .with_label(
            Label::new(("input", span.clone()))
            .with_message(&message)
            .with_color(label_color)
        )
        .finish()
        .print(("input", Source::from(source)))
        .unwrap()
}

pub fn report_compiler_all(source: &str, errors: &[HypercError]) {
    if errors.is_empty() { return }
    for error in errors {
        report_compiler_error(source, error);
    }
    std::process::exit(exit_code(&errors[0]))
}

pub fn exit_code(error: &HypercError) -> i32 {
    match error {
        HypercError::LexerError     { .. } => 1,
        HypercError::ParseError     { .. } => 2,
        HypercError::ResolveError   { .. } => 3,
        HypercError::TypeError      { .. } => 4,
        HypercError::CompileError   { .. } => 5,
        HypercError::BuildError     { .. } => 6,
        HypercError::SetupError     { .. } => 7
    }
}