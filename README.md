# hyperc

**hyperc** is a compiler for the Hyper programming language, written in Rust.

It compiles `.hr` source files to native machine code using LLVM and Clang.

## Features

- Primitive types: `int`, `float`, `str`, `char`, `bool`
- Mutable and immutable variables: `mut` and `const`
- Functions
- Conditional statements: `if` / `else`
- Loops: `while` and `for`
- Output with `print`
- Automatic conversion between `int` and `float` where supported by the language
- Lexer, parser, name resolution, type checking, and native code generation

## Requirements

To build hyperc from source, you need:

- [Rust](https://www.rust-lang.org/tools/install) with Cargo
- LLVM 22 development libraries
- Clang 22
- A C/C++ toolchain and the system libraries required by LLVM

The exact package names and installation steps depend on your operating system.

> **Note:** `cargo build` builds hyperc; it does not install LLVM. Install the required LLVM version and development libraries before building.

## Building from source

Clone the repository:

```bash
git clone https://github.com/rub1-618/hyperc.git
cd hyperc
```

Build the compiler:

```bash
cargo build --release
```

The compiled executable will be located at:

```text
target/release/hyperc
```

To run the test suite:

```bash
cargo test --release
```

## Usage

hyperc provides four main commands: `setup`, `run`, `build`, and `check`.

### Run a program

Compile and execute a source file:

```bash
hyperc run test.hr
```

### Build a program

Compile a source file without running it:

```bash
hyperc build test.hr
```

### Check a program

Check a source file without producing and executing a program:

```bash
hyperc check test.hr
```

### Setup

Run the setup command:

```bash
hyperc setup
```

For the complete list of commands and options, use:

```bash
hyperc --help
hyperc run --help
hyperc build --help
hyperc check --help
```

## Command-line options

Depending on the command, hyperc supports options such as:

| Option | Description |
| --- | --- |
| `-o`, `--out` | Specify the output directory |
| `--show-tokens` | Display lexer tokens |
| `--show-ast` | Display the abstract syntax tree |
| `--show-ir` | Display LLVM IR |
| `--color` | Configure colored output: `auto`, `always`, or `never` |
| `-v`, `--verbose` | Enable verbose output |
| `--emit` | Select the output format for `build`; defaults to `obj` |

Use the command-specific help output to see which options are available for each command.

## Example

Create a file named `test.hr` containing a Hyper program, then run:

```bash
hyperc run test.hr
```

The exact syntax of a program depends on the Hyper language grammar. See the `tests/` directory for examples used by the project.

## Windows

hyperc can be built and run in a Linux environment on Windows using **WSL2** or **Docker Desktop with the WSL2 backend**.

The Docker approach lets you run the Linux compiler without installing Rust and LLVM directly in Windows.

### Requirements

- [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- WSL2 enabled, if required by your Docker Desktop setup
- Git, if you want to clone the repository

### Build the Docker image

From the repository root, run:

```bash
docker build -t hyperc:0.3.5 .
```

### Run a Hyper source file

Place your `.hr` file in the current directory. Then run the following command in PowerShell:

```powershell
docker run --rm -v "${PWD}:/work" hyperc:0.3.5 run /work/test.hr
```

The command mounts the current directory inside the container at `/work` and runs `test.hr` with hyperc.

This command assumes the image's entry point is configured to invoke the `hyperc` executable.

## Error handling

hyperc reports errors encountered during lexical analysis, parsing, name resolution, type checking, compilation, and building.

The current compiler error exit codes are:

| Code | Error |
| ---: | --- |
| `1` | Lexer error |
| `2` | Parse error |
| `3` | Name resolution error |
| `4` | Type error |
| `5` | Compilation error |
| `6` | Build error |

Command-line usage errors and errors returned by an executed program may use different exit codes.

## Project structure

```text
hyperc/
├── src/          # Compiler source code
├── tests/        # Tests and language examples
├── Cargo.toml    # Rust package configuration
├── Cargo.lock    # Locked dependency versions
└── README.md
```

## Dependencies

hyperc is written in Rust and uses:

- [Clap](https://crates.io/crates/clap) for command-line argument parsing
- [Ariadne](https://crates.io/crates/ariadne) for diagnostic reporting
- [Inkwell](https://crates.io/crates/inkwell) for LLVM bindings

The compiler is configured to use LLVM 22 through Inkwell.

## Contributing and testing

Contributions and bug reports are welcome.

Before submitting changes, run:

```bash
cargo test --release
cargo build --release
```

## License

This project is licensed under the MIT License. See the LICENSE file for details.