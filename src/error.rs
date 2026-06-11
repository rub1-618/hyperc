use std::ops::Range;
use ariadne::{ Report, ReportKind, Label, Source };

#[derive(Debug, Clone)]
pub struct ParseError {
     pub span:  Range<usize>,
     pub message: String,
}

#[derive(Debug, Clone)]
pub struct TypeError {
     pub span:  Range<usize>,
     pub message: String,
}

pub fn report_parse(source: &str, error: &ParseError) {
    Report::build(ReportKind::Error, ("input", error.span.clone()))
    .with_message(&error.message)
    .with_label(
        Label::new(("input", error.span.clone()))
        .with_message(&error.message)
    )
    .finish()
    .print(("input", Source::from(source)))
    .unwrap()
}

pub fn report_type(source: &str, error: &TypeError) {
    Report::build(ReportKind::Error, ("input", error.span.clone()))
    .with_message(&error.message)
    .with_label(
        Label::new(("input", error.span.clone()))
        .with_message(&error.message)
    )
    .finish()
    .print(("input", Source::from(source)))
    .unwrap()
}