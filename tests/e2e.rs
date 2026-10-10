use std::{fs, path::{Path, PathBuf}, process::{Command, Output}};

fn run_test(path: &str) -> (Output, String) {
    let compiler = env!("CARGO_BIN_EXE_hyperc");
    let out_name = Path::new(path).file_stem().unwrap().to_str().unwrap();

    let mut cmd = Command::new(compiler);
    cmd.args([
        "run", path,
        "-o", out_name
    ]);

    let status = cmd.status().unwrap();
    assert!(status.success());
    
    let obj = format!("./tests/fixtures/{}.o", out_name);
    let out = format!("./tests/fixtures/{}", out_name);
    let output = Command::new(&out).output().unwrap();

    let stdout = 
     String::from_utf8(output.stdout.clone()).unwrap();

    match fs::remove_file(out) {
        Ok(_) => {}
        Err(e) => panic!("Unable to delete a binary file: {}", e)
    }

    match fs::remove_file(obj) {
        Ok(_) => {}
        Err(e) => panic!("Unable to delete an obj file: {}", e)
    }

    (output, stdout)
}

fn asm_test(path: &str) {
    let compiler = env!("CARGO_BIN_EXE_hyperc");
    let out_name = Path::new(path).file_stem().unwrap().to_str().unwrap();

    let mut cmd = Command::new(compiler);
    cmd.args([
        "build", path,
        "-o", out_name,
        "--emit", "asm"
    ]);

    let status = cmd.status().unwrap();
    assert!(status.success());

    let out = format!("./tests/fixtures/{}", out_name);
    let asm = PathBuf::from(format!("./tests/fixtures/{}.s", out_name));
    assert!(&asm.exists());

    let content = match fs::read_to_string(&asm) {
        Ok(s) => s,
        Err(e) => panic!("Unable to read an asm file: {}", e)
    };

    assert!(!content.is_empty());
    assert!(content.contains("main"));

    match fs::remove_file(out) {
        Ok(_) => {}
        Err(e) => panic!("Unable to delete a binary file: {}", e)
    }

    match fs::remove_file(asm) {
        Ok(_) => {}
        Err(e) => panic!("Unable to delet an asm file: {}", e)
    }
}

#[test]
fn test_run_parade() {
    let path = "./tests/fixtures/test_parade.hr";
    let (output, stdout) = run_test(path);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "-10\nhi\n0.000000\nfalse\nc\n");
    asm_test(path);
}

// TODO
// #[test]
// fn test_run_exit_code() {
//     let path = "./tests/fixtures/test_exit_code.hr";
//     let (output, stderr) = run_test(path);
//     assert_eq!(stderr, "42".to_string());
//     asm_test(path);
// }

#[test]
fn test_run_if_else() {
    let path = "./tests/fixtures/test_if_else.hr";
    let (output, stdout) = run_test(path);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "3.140000\n");
    asm_test(path);
}

#[test]
fn test_run_while() {
    let path = "./tests/fixtures/test_while.hr";
    let (output, stdout) = run_test(path);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "4\n6\n");
    asm_test(path);
}

#[test]
fn test_run_for() {
    let path = "./tests/fixtures/test_for.hr";
    let (output, stdout) = run_test(
        path
    );
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "0\n2\n");
    asm_test(path);
}

#[test]
fn test_run_func() {
    let path = "./tests/fixtures/test_func.hr";
    let (output, stdout) = run_test(path);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "5\n");
    asm_test(path);
}

#[test]
fn test_run_struct() {
    let path = "./tests/fixtures/test_struct.hr";
    let (output, stdout) = run_test(path);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "5\nfalse\n3.140000\n");
    asm_test(path);
}

#[test]
fn test_run_impl() {
    let path = "./tests/fixtures/test_impl.hr";
    let (output, stdout) = run_test(path);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "true\n1\n5\n7\n");
    asm_test(path);
}

#[test]
fn test_run_enum() {
    let path = "./tests/fixtures/test_enum.hr";
    let (output, stdout) = run_test(path);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(stdout, "red\n");
    asm_test(path);
}