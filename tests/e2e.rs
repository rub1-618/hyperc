use std::{path::Path, process::{Command, Output}};

fn run_test(path: &str) -> (Output, String) {
    let compiler = env!("CARGO_BIN_EXE_hyperc");
    let out_name = Path::new(path).file_stem().unwrap().to_str().unwrap();

    let mut cmd = Command::new(compiler);
    cmd.args([
        "run", path,
        "-o", out_name
    ]);

    let status = cmd.status().unwrap();
    eprintln!("{}", status);
    assert!(status.success());
    
    let out = format!("./tests/fixtures/{}", out_name);
    let output = Command::new(out).output().unwrap();

    let stdout = 
     String::from_utf8(output.stdout.clone()).unwrap();

    (output, stdout)
}

#[test]
fn test_parade() {
    let (output, stdout) = run_test(
        "./tests/fixtures/test_parade.hr"
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "-10\nhi\n0.000000\nfalse\nc\n");
}

#[test]
fn test_exit_code() {
    let (output, _) = run_test(
        "./tests/fixtures/test_exit_code.hr"
    );
    assert_eq!(output.status.code(), Some(42));
}

#[test]
fn test_if_else() {
    let (output, stdout) = run_test(
        "./tests/fixtures/test_if_else.hr"
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "3.140000\n");
}

#[test]
fn test_while() {
    let (output, stdout) = run_test(
        "./tests/fixtures/test_while.hr"
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "4\n6\n");
}

#[test]
fn test_for() {
    let (output, stdout) = run_test(
        "./tests/fixtures/test_for.hr"
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "0\n2\n");
}

#[test]
fn test_func() {
    let (output, stdout) = run_test(
        "./tests/fixtures/test_func.hr"
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "5\n");
}

#[test]
fn test_struct() {
    let (output, stdout) = run_test(
        "./tests/fixtures/test_struct.hr"
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "5\nfalse\n3.140000\n");
}

#[test]
fn test_impl() {
    let (output, stdout) = run_test(
        "./tests/fixtures/test_impl.hr"
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "true\n1\n5\n7\n");
}

#[test]
fn test_enum() {
    let (output, stdout) = run_test(
        "./tests/fixtures/test_enum.hr"
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "red\n");
}