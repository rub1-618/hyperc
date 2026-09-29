# hyperc

# hyperc - a compiler for my statically typed programming language "hyper", which is compiled to native code with LLVM.

A taste of hyper:

```
// test_parade.hr
func main() -> int {
    print(-10);
    print("hi");
    print(0.0);
    print(!true);
    print('c');
    return 0;
}

```

```
  > hyperc test_parade.hr

```

```
  > Out: 
    -10
    hi
    0.000000
    false
    c
```

# -- FEATURES --
                                                                  
# Variable types:
- int
- float
- str
- char
- bool
# Variable kinds:
- mut
- const
# 'func' functions
# if / else
# while / for loops
# print statement

<!-- P.S. on the compilations state if expression has one float and one int, int automatically converts to float value. -->

# All of the compilation goes straight to the native exe in the same path folder, you have chosen in first command.

# -- INSTALLATION --
                                                                                        
# after installing the compiler files in my repository you may consider using this command:
> cargo install --path .
# You should also have LLVM 22+ version and a clang/gcc installed. You can get the correct LLVM version with a regular:
> cargo build

# ! Unix-based systems are fully supported, while on Windows works only wia WSL.

# -- USAGE --
                                        
# The commands are:
> hyperc                                <!-- Compiles a one-line program for stdin. -->  
> hyperc <script>                       <!-- Runs the file specified. -->  
> hyperc <script> [-o name]             <!-- Lets you name an .o and .exe files, like in gcc. -->
> hyperc <script> [-d | --debug]      <!-- Prints IR to stdout. -->
> hyperc -V | > hyperc --version        <!-- Shows you the current version of the hyperc. -->
> hyperc -h | > hyperc --help           <!-- More detailed information of possible commands. -->

# Exit codes:
- 0  // Success
- 64 // Unknown argument
- 65 // LexerError
- 66 // ParseError (parser)
- 67 // ParseError (resolver)
- 68 // TypeError
- 69 // CompileError

# -- SYNTAX --
                                                    
# ! Main function in "hyperc" is a place where all code starts running, just like C++ or Rust. You cannot have top-level variables, only functions, structs, impls and enums are allowed there:

<!-- main function example -->
func main() -> int {
    ...
    return 0;
}

# Functions:

<!-- The exampe of a regular sum() function: -->
func sum(a: int, b: int) -> int {
    print( a + b );
    return ( a + b );
}

# Variables:

<!-- Declaring and assigning a mutable variable: -->
let mut x: int = 5;
x = 6; // ok

<!-- Declaring and assigning a constant variable: -->
let const y: int = 5;
y = 6; // error, you cannot assign to a const.

<!-- Declaring a variable to a function is also allowed: -->
let const sum_3_4: int = sum(3, 4); // from the sum() function, the result is 7.

# if / else conditions:
<!-- P.s. else if is planned for future updates. -->

if (5 > 3) {
    print("5 is bigger than 3");
} else {
    print("5 is less than 3");
}

> Output: 5 is bigger than 3

# while / for loops:

<!-- while: -->
let mut x: int = 0;
while (x < 2) {
    x = x + 1;
    print(x);
}

> Output: 1 2

<!-- for: -->

for (let mut i: int = 0; i < 4; i = i + 2) {
    print(i);
}

> Output: 0 2

# structs / impls

struct Point {
    x: int,
    y: int,
}

impl Point {

    func get_x () -> int {
        return self.x;
    }

    func set_x (v: int) {
        self.x = v;
    }
}

func main() -> int {
    let mut p: Point = Point { x: 1, y: 2 };
    p.set_x(4);
    let const res: int = p.get_x();
    print(res);

    return 0;
}

> Output: 4

# Basic comments: 

// your text

# -- BASIC RULES --

- You must use main function in order for your code to be executed.
- For top-level you can use only functions, structs, impls and enums.
- In v0.3 you should declare your struct / function and only then use it.
- In impls 'self' is implicit (for now), you cannot put it into method parameters, but you can call for example 'self.x' from anywhere inside the impl.
- To make a char with a ' symbol you can just use let const c: char = ''';. there are no backslash escapes for chars. (yet)
- Printing an enum gives you a position of a variant.
- You cannot print structs. (yet)
- Bool printing gives you true/false as an output string.
- You cannot compare strings. (but you can compare chars)

# -- UNDER THE HOOD --

# your .hr file
       |
       ▼
#    lexer          <!-- Your text converts to tokens. -->
       |
       ▼
#    parser         <!-- Tokens go through AST. -->
       |
       ▼
#   resolver        <!-- Checking undefined variables, redeclarement,  -->
       |            <!-- top-level rules, has_main, block checking and constants checking. -->
       |    
       ▼
# type checker      <!-- Checking variables and return types, missing return, calls. -->
       |
       ▼
#   codegen         <!-- AST -> LLVM IR (inkwell) -> verification -->
       |
       ▼
#    clang -> out.o -> .exe

#   error.rs <!-- Error handling -->

# Crates used for creation:
- clap = "4.6.6"    <!-- For fluid CLI experience. -->

- ariadne = "0.6"   <!-- For beautiful error handling like this: -->

Error: Cannot redeclare const variable.
   ╭─[ input:3:5 ]
   │
 3 │     x = 5;
   │     ┬  
   │     ╰── Cannot redeclare const variable.
───╯

- inkwell = { version="0.9", features=["llvm22-1"] } <!-- To make my own PL by safely wrapping the llvm-sys. -->

# -- TESTS --
                                          
# Totally 117 tests were made and successfully completed. 108 unit tests and 9 e2e tests.
# You can check them out with these commands:
> cargo test                <!-- To check all 117 tests. -->
> cargo test --test e2e     <!-- Only to test the e2e. -->

# -- ROADMAP -- 

# v0.4+
- str variables (let mut s: str = "...";)
- type inference ( let x = 5; // const int)
- self in method params
- else if
- borrow checker
- binding enum + lvalue_root-enum
- associated functions
- forward-calls for functions
- struct printing

# Planned
- std lib

# Ideas / Research
- mut+

```
let mut+ x: int = 7;
x = "hi" // allows to change types

// requires relaxing static typing for this kind
 
```

- dynamic blocks with unified environments

```
let mut a = 10;

block alpha {
    let const a: int = 5;
}

print(a); // Out: 10

block alpha {
    print(a); // out: 5;
}

// these blocks are like a fully separate scope for your file

```
- REPL ( console compiler )