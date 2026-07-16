use std::{path::Path, process::{Command, Output}};

fn run_hr(path: &str) -> (Output, String) {
    let compiler = env!("CARGO_BIN_EXE_hyperc");

    let cmd_path = Path::new(path).file_stem().unwrap().to_str().unwrap();

    let mut cmd = Command::new(compiler);
    cmd.arg(path);
    cmd.arg("-o");
    cmd.arg(cmd_path);
    let status = cmd.status().unwrap();
    assert!(status.success());
    let out = format!("tests/fixtures/{}", cmd_path);
    let cmd_out = Command::new(out)
    .output().unwrap();

    let std_out = String::from_utf8(cmd_out.stdout.clone()).unwrap();

    (cmd_out, std_out)
}

#[test]
fn test_parade() {
    let (cmd_out, std_out) = run_hr("tests/fixtures/test_parade.hr");
    assert_eq!(std_out, "-10\nhi\n0.000000\nfalse\nc\n"); 
    assert_eq!(cmd_out.status.code(), Some(0));  
}

#[test]
fn test_exit_code() {
    let (cmd_out, _) = run_hr("tests/fixtures/test_exit_code.hr");
    assert_eq!(cmd_out.status.code(), Some(42));
}

#[test]
fn test_if_else() {
    let (cmd_out, std_out) = run_hr("tests/fixtures/test_if_else.hr");
    assert_eq!(std_out, "-10\n");
    assert_eq!(cmd_out.status.code(), Some(0));  
}

#[test]
fn test_while() {
    let (cmd_out, std_out) = run_hr("tests/fixtures/test_while.hr");
    assert_eq!(std_out, "4\n6\n");
    assert_eq!(cmd_out.status.code(), Some(0));  
}

#[test]
fn test_for() {
    let (cmd_out, std_out) = run_hr("tests/fixtures/test_for.hr");
    assert_eq!(std_out, "0\n2\n");
    assert_eq!(cmd_out.status.code(), Some(0));
}

#[test]
fn test_func() {
    let (cmd_out, std_out) = run_hr("tests/fixtures/test_func.hr");
    assert_eq!(std_out, "7\n");
    assert_eq!(cmd_out.status.code(), Some(0));
}

#[test]
fn test_struct() {
    let (cmd_out, std_out) = run_hr("tests/fixtures/test_struct.hr");
    assert_eq!(std_out, "1\nfalse\n2.500000\n");
    assert_eq!(cmd_out.status.code(), Some(0));
}

#[test]
fn test_impl() {
    let (cmd_out, std_out) = run_hr("tests/fixtures/test_impl.hr");
    assert_eq!(std_out, "true\n1\n5\n7\n");
    assert_eq!(cmd_out.status.code(), Some(0));
}

#[test]
fn test_enum() {
    let (cmd_out, std_out) = run_hr("tests/fixtures/test_enum.hr");
    assert_eq!(std_out, "red\n");
    assert_eq!(cmd_out.status.code(), Some(0));
}