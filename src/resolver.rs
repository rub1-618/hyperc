use crate::error::HypercError;
use crate::token::Token;
use crate::ast::{Expr, Stmt, VarKind, VarType};
use crate::support::{expr_span, stmt_span};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub enum Binding {
    Variable {
        ready: bool,
        kind: VarKind
    },
    Func
}

#[derive(Debug, Clone)]
pub enum TypeInfo {
    Struct {
        fields: Vec<(String, VarType)>
    },
    Enum {
        variants: Vec<String>
    }
}

// ! -- resolver --

#[derive(Debug, Clone)]
pub struct  Resolver {
    scopes: Vec<HashMap<String, Binding>>,
    types: HashMap<String, TypeInfo>,
    impls: HashMap<String, Vec<String>>,
    in_method: bool,
    pub errors: Vec<HypercError>
}

impl Resolver {

    pub fn new() -> Self {
        Resolver { 
            scopes: Vec::new(), 
            types: HashMap::new(),
            impls: HashMap::new(),
            in_method: false, 
            errors: Vec::new() 
        }
    }

    pub fn resolve(&mut self, stmts: &[Stmt]) {
        self.begin_scope();
        let mut has_main = false;
        for stmt in stmts {
            match stmt {
                Stmt::Func { name, .. } => {
                    if &name.lexeme == "main" {
                        has_main = true;
                    }
                    self.resolve_stmt(stmt);
                }

                Stmt::Struct { .. } | Stmt::Impl { .. }
                | Stmt::Enum { .. } => {
                    self.resolve_stmt(stmt);
                }

                _ => self.errors.push(HypercError::ResolveError {
                    span: stmt_span(stmt),
                    message: "Only functions, structs, impls and enums are top-level-supported.".to_string()
                })
            }
        }
        if !has_main {
            self.errors.push(HypercError::ResolveError {
                span: 0..0,
                message: "'main' function not found.".to_string()
            });
        }
    }

    fn resolve_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            
            Stmt::Expression { value } => {
                self.resolve_expr(value);
            }

            Stmt::Print { value } => {
                self.resolve_expr(value);
            }
        
            Stmt::Let { 
                name, value,
                var_kind, var_type
            } => {
                self.declare_variable(name, var_kind.clone());
                self.check_type_exists(var_type);
                self.resolve_expr(value);
                self.define(name);
            }

            Stmt::Assign { target, value } => {
                self.resolve_expr(value);
                match &**target {
                    Expr::Variable { name } => {
                        match self.get_binding(&name) {
                            Some(binding) => {
                                match binding {
                                    Binding::Variable { kind, .. } => {
                                        if *kind == VarKind::Const {
                                            self.errors.push(HypercError::ResolveError {
                                                span: name.start..name.end,
                                                message: "Cannot assign to a const variable.".to_string()
                                            })
                                        }
                                    }
                                    Binding::Func => self.errors.push(HypercError::ResolveError {
                                        span: name.start..name.end,
                                        message: "Cannot assign function or its arguments.".to_string()
                                    })
                                }
                            }
                            None => self.errors.push(HypercError::ResolveError {
                                span: name.start..name.end,
                                message: "Variable not found.".to_string()
                            })
                        }
                    }

                    Expr::Get { object, .. } => {
                        self.resolve_expr(object);
                        let root_name = Self::get_lvalue_root(target);
                        match root_name {
                            Some(rn) => {
                                if rn.lexeme != "self" {
                                    match self.get_binding(rn) {
                                        Some(binding) => {
                                            match binding {
                                                Binding::Variable { kind, .. } => {
                                                    if *kind == VarKind::Const {
                                                        self.errors.push(HypercError::ResolveError {
                                                            span: rn.start..rn.end,
                                                            message: "Cannot assign to const field.".to_string()
                                                        })
                                                    }
                                                }
                                                Binding::Func => self.errors.push(HypercError::ResolveError {
                                                    span: rn.start..rn.end,
                                                    message: "Cannot assign to a method.".to_string()
                                                })
                                            }
                                        }
                                        None => self.errors.push(HypercError::ResolveError {
                                            span: rn.start..rn.end,
                                            message: "Field not found.".to_string()
                                        }),
                                    }
                                }
                            }
                            None => self.errors.push(HypercError::ResolveError {
                                span: expr_span(target),
                                message: "Invalid assignment target.".to_string()
                            }),
                        }
                    }
                    _ => self.errors.push(HypercError::ResolveError {
                        span: expr_span(target),
                        message: "Invalid assignment target.".to_string()
                    }),
                }
            }

            Stmt::Block { statements } => {
                self.resolve_stmts(statements);
            }

            Stmt::If { 
                condition, 
                then_branch, 
                else_branch 
            } => {
                self.resolve_expr(condition);
                self.resolve_stmt(then_branch);
                if let Some(else_stmt) = else_branch {
                    self.resolve_stmt(else_stmt);
                };
            }

            Stmt::While { condition, statements } => {
                self.resolve_expr(condition);
                self.resolve_stmt(statements);
            }

            Stmt::For { 
                initializer, condition, 
                increment, statements 
            } => {
                self.begin_scope();
                let result = ( || {
                    if let Some(init) = initializer {self.resolve_stmt(init);}
                    if let Some(cond) = condition   {self.resolve_expr(cond);}
                    if let Some(incr) = increment   {self.resolve_stmt(incr);}
                    self.resolve_stmt(statements);
                })();
                self.end_scope();
                result
            }

            Stmt::Func { 
                name, args,
                statements, return_type 
            } => {
                self.resolve_func_body(name, args, statements, return_type, false);
            }

            Stmt::Return { value } => {
                if let Some(expr) = value {
                    self.resolve_expr(expr);
                }
            }

            Stmt::Struct { name, fields } => {
                if self.types.contains_key(&name.lexeme) {
                    self.errors.push(HypercError::ResolveError {
                        span: name.start..name.end,
                        message: "This type is already declared.".to_string()
                    });
                }
                let mut fields_vec = vec![];
                for ( t, v ) in fields {
                    if let VarType::Named(tok) = v {
                        if tok.lexeme == name.lexeme {
                            self.errors.push(HypercError::ResolveError {
                                span: tok.start..tok.end,
                                message: "This type is already declared.".to_string()
                            });
                        }
                        if !self.types.contains_key(&tok.lexeme) {
                            self.errors.push(HypercError::ResolveError {
                                span: tok.start..tok.end,
                                message: "This type is already declared.".to_string()
                            });
                        }
                    }
                    if fields_vec.iter().any(|(n,_)| n == &t.lexeme) {
                        self.errors.push(HypercError::ResolveError {
                            span: t.start..t.end,
                            message: "This field is already declared.".to_string()
                        });
                    }
                    fields_vec.push((t.lexeme.clone(), v.clone()));
                }
                self.types.insert(name.lexeme.clone(), TypeInfo::Struct { fields: fields_vec });
            }

            Stmt::Impl { name, methods } => {
                if !self.types.contains_key(&name.lexeme) {
                    self.errors.push(HypercError::ResolveError {
                        span: name.start..name.end,
                        message: "Type not found.".to_string()
                    });
                }

                let mut method_names = vec![];
                for method in methods {
                    match method {
                        Stmt::Func { 
                            name, args,
                            statements, return_type
                        } => {
                            self.resolve_func_body(&name, &args, &statements, &return_type, true);
                            method_names.push(name.lexeme.clone());
                        }

                        _ => self.errors.push(HypercError::ResolveError {
                            span: name.start..name.end,
                            message: "Expected method.".to_string()
                        })
                    }
                }
                self.impls.insert(name.lexeme.clone(), method_names);
            }

            Stmt::Enum { name, variants } => {
                if self.types.contains_key(&name.lexeme) {
                    self.errors.push(HypercError::ResolveError {
                        span: name.start..name.end,
                        message: "This type is already declared.".to_string()
                    });
                }

                let mut variants_vec = vec![];
                for variant in variants {
                    if variants_vec.contains(&variant.lexeme) {
                        self.errors.push(HypercError::ResolveError {
                            span: variant.start..variant.end,
                            message: "This variant is already declared.".to_string()
                        });
                    }
                    variants_vec.push(variant.lexeme.clone());
                }
                self.types.insert(name.lexeme.clone(), TypeInfo::Enum { variants: variants_vec });
            }
        }
    }

    fn resolve_expr(&mut self, expr: &Expr) {
        match expr {
            
            Expr::Call { callee, arguments, .. } => {
                for argument in arguments {
                    self.resolve_expr(argument);
                }
                match &**callee {
                    Expr::Variable { name } => {
                        match self.get_binding(&name) {
                            Some(_) => {},
                            None => self.errors.push(HypercError::ResolveError {
                                span: expr_span(callee),
                                message: "Function not found.".to_string()
                            })
                        }
                    }

                    Expr::Get { object, .. } => {
                        self.resolve_expr(&object);
                    }

                    Expr:: Path { type_name, item } => {
                        match self.types.get(&type_name.lexeme) {
                            Some(TypeInfo::Struct { .. }) => {
                                if let Some(methods) = self.impls.get(&type_name.lexeme) {
                                    if !methods.contains(&item.lexeme) {
                                        self.errors.push(HypercError::ResolveError {
                                            span: item.start..item.end,
                                            message: "Method not found.".to_string()
                                        })
                                    }
                                } else {
                                    self.errors.push(HypercError::ResolveError {
                                        span: type_name.start..type_name.end,
                                        message: "This type doesn't have an impl.".to_string()
                                    })
                                }
                            }
                            _ => self.errors.push(HypercError::ResolveError {
                                span: type_name.start..type_name.end,
                                message: "Impl type not found.".to_string()
                            }),
                        }
                    }

                    _ => self.errors.push(HypercError::ResolveError {
                        span: expr_span(callee),
                        message: "Not a function.".to_string()
                    }),
                }
            }

            Expr::Variable { name } => {
                match self.get_binding(name) {
                    Some(Binding::Variable { ready, .. }) => {
                        if !*ready {
                            self.errors.push(HypercError::ResolveError {
                                span: name.start..name.end,
                                message: "Variable is used in self declarement.".to_string()
                            });
                        }
                    }
                    Some(Binding::Func) => {}
                    None => {
                        self.errors.push(HypercError::ResolveError {
                            span: name.start..name.end, 
                            message: "Variable or method not found.".to_string()
                        });
                    }
                }
            }

            Expr::StructLit { name, fields } => {
                match self.types.get(&name.lexeme) {
                    Some(TypeInfo::Enum { .. }) => {
                        self.errors.push(HypercError::ResolveError {
                            span: name.start..name.end,
                            message: "Expected struct, got enum.".to_string()
                        });
                    }
                    Some(TypeInfo::Struct { .. }) => {
                        let mut field_hash = HashSet::new();
                        for (tok, expr) in fields {
                            match field_hash.insert(tok.lexeme.clone()) {
                                true => self.resolve_expr(expr),
                                false => self.errors.push(HypercError::ResolveError {
                                    span: tok.start..tok.end,
                                    message: "This field is already declared.".to_string()
                                })
                            }
                        }
                    }
                    None => self.errors.push(HypercError::ResolveError {
                        span: name.start..name.end,
                        message: "Type not found.".to_string()
                    })
                }
            }

            Expr::Get { object, .. } => {
                self.resolve_expr(object);
            }

            Expr::Path { type_name, item } => {
                match self.types.get(&type_name.lexeme) {
                    
                    Some(TypeInfo::Enum { variants }) => {
                        if !variants.contains(&item.lexeme) {
                            self.errors.push(HypercError::ResolveError {
                                span: item.start..item.end,
                                message: "Variant not found.".to_string()
                            })
                        }
                    }

                    Some(TypeInfo::Struct { .. }) => {
                        self.errors.push(HypercError::ResolveError {
                            span: type_name.start..type_name.end,
                            message: "You can path only to methods with this type.".to_string()
                        })
                    }

                    None => self.errors.push(HypercError::ResolveError {
                        span: type_name.start..type_name.end,
                        message: "Type not found.".to_string()
                    }),
                }
            }

            Expr::SelfExpr { self_tok } => {
                if !self.in_method {
                    self.errors.push(HypercError::ResolveError {
                        span: self_tok.start..self_tok.end,
                        message: "'self' is outside of a method.".to_string()
                    });
                }
            }

            _ => {}

        }
    }

    fn resolve_func_body(&mut self, name: &Token, args: &Vec<(Token, VarType)>,
    statements: &Box<Stmt>, return_type: &Option<VarType>, is_method: bool) {
        self.declare_func(name);
        self.define(name);
        let prev_state = self.in_method;
        self.in_method = is_method;
        if let Some(rt) = return_type {
            self.check_type_exists(rt);
        }
        self.begin_scope();
        for ( .., v ) in args {
            self.check_type_exists(v);
        }
        let result = ( || {
            for arg in args {
                self.declare_func(&arg.0);
                self.define(&arg.0);
            }
            self.resolve_stmt(statements);
        })();
        self.end_scope();
        self.in_method = prev_state;
        result
    }

    fn declare_variable(&mut self, name: &Token, var_kind: VarKind) {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(&name.lexeme) {
                self.errors.push(HypercError::ResolveError {
                    span: name.start..name.end,
                    message: "Variable is already declared.".to_string()
                });
            }
            scope.insert(
                name.lexeme.clone(), 
                Binding::Variable { 
                    ready: false, 
                    kind: var_kind 
                }
            );
        }
    }

    fn declare_func(&mut self, name: &Token) {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(&name.lexeme) {
                self.errors.push(HypercError::ResolveError {
                    span: name.start..name.end,
                    message: "Already declared.".to_string()
                });
            }
            scope.insert(
                name.lexeme.clone(), 
                Binding::Func
            );
        }
    }

    fn define(&mut self, name: &Token) {
        if let Some(scope) = self.scopes.last_mut() {
            if let Some(binding) = scope.get_mut(&name.lexeme) {
                match binding {
                    Binding::Variable { ready, .. } => {
                        *ready = true;
                    }
                    Binding::Func => {}
                }
            }
        }
    }

    fn get_binding(&self, name: &Token) -> Option<&Binding> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(&name.lexeme) {
                return Some(binding);
            }
        }
        None
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    fn check_type_exists(&mut self, var_type: &VarType) {
        if let VarType::Named(tok) = var_type {
            if !self.types.contains_key(&tok.lexeme) {
                self.errors.push(HypercError::ResolveError {
                    span: tok.start..tok.end,
                    message: "Type not found.".to_string()
                });
            }
        }
    }

    fn get_lvalue_root(expr: &Expr) -> Option<&Token> {
        match expr {
            Expr::Get { object, .. } => {
                match &**object {
                    Expr::SelfExpr { self_tok, .. } => Some(&self_tok),
                    _ => Self::get_lvalue_root(object),
                }
            },
            Expr::Variable { name } => Some(name),
            _ => None
        }
    }

    pub fn resolve_stmts(&mut self, stmts: &[Stmt]) {
        self.begin_scope();
        for stmt in stmts {
            self.resolve_stmt(stmt);
        }
        self.end_scope();
    }

    pub fn get_types(self) -> HashMap<String, TypeInfo> {
        self.types
    }
}

#[cfg(test)]
mod tests {
    use crate::error::HypercError;
    use crate::*;

    fn resolve_source(src: &str) -> Result<(), Vec<HypercError>> {
        let mut lexer = lexer::Lexer::new(src.to_string());
        let tokens = lexer.scan_tokens();
        let mut parser = parser::Parser::new(tokens);
        let stmts = parser.parse();
        let mut resolver = resolver::Resolver::new();
        let result = resolver.resolve_stmts(&stmts);
        if !resolver.errors.is_empty() {
            return Err(resolver.errors);
        }
        Ok(result)
    }

    fn resolve_source_with_main(src: &str) -> Result<(), Vec<HypercError>> {
        let mut lexer = lexer::Lexer::new(src.to_string());
        let tokens = lexer.scan_tokens();
        let mut parser = parser::Parser::new(tokens);
        let stmts = parser.parse();
        let mut resolver = resolver::Resolver::new();
        let result = resolver.resolve(&stmts);
        if !resolver.errors.is_empty() {
            return Err(resolver.errors);
        }
        Ok(result)
    }

    // ! -- exprs --

    #[test]
    fn test_resolver_call_expr_ok() {
        // func
        let result = resolve_source("
            func foo() {}
            foo();
        ");
        assert!(result.is_ok());

        // path
        let result = resolve_source("
            struct Struct {} impl Struct { func new() {} }
            Struct::new();
        ");
        assert!(result.is_ok());

        // get
        let result = resolve_source("
            struct Struct {} impl Struct { func new() {} }
            let mut s: Struct = Struct {};
            s.new();
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_variable_expr_ok() {
        let result = resolve_source("let const x: int = 5; x;");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_structlit_expr_ok() {
        let result = resolve_source("
            struct Type { y: int }
            let const x: Type = Type { y: 5 };
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_get_expr_ok() {
        let result = resolve_source("
            struct Type { y: int }
            let mut x: Type = Type { y: 5 };
            x.y = 5;
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_rseolver_path_expr_ok() {
        // enum
        let result = resolve_source("
            enum Color { Red, Green, Blue }
            let mut x: Color = Color::Red;
            Color::Blue;
        ");
        assert!(result.is_ok());
        
        // impl
        let result = resolve_source("
            struct Type { x: int }
            impl Type { func new() {} }
            let fluid x: Type = Type::new();
            Type::new();
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_self_expr_ok() {
        let result = resolve_source("
            struct Type { y: int }
            impl Type {
                func new() {
                    print(self.y);
                }
            }
        ");
        assert!(result.is_ok());
    }

    // ! -- stmts --

    #[test]
    fn test_resolver_expr_stmt_ok() {
        let result = resolve_source("5 + 1;");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_print_stmt_ok() {
        let result = resolve_source("print(\"Hello world!\");");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_let_declaration_stmt_ok() {
        let result = resolve_source("let fluid x: int = 5;");
        assert!(result.is_ok());

        // get
        let result = resolve_source("
            struct Type { x: int }
            let const y: Type = Type { x: 5 };
            let fluid x: int = y.x;
        ");
        assert!(result.is_ok());

        // path
        let result = resolve_source("
            enum Color { Red, Green, Blue }
            let fluid x: Color = Color::Red;
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_assign_stmt_ok() {
        let result = resolve_source("let fluid x: int = 5; x = 10;");
        assert!(result.is_ok());
        
        //get
        let result = resolve_source("
            struct Type { x: int }
            let const y: Type = Type { x: 5 };
            let fluid x: int = 5;
            x = y.x;
        ");
        assert!(result.is_ok());

        // path
        let result = resolve_source("
            enum Color { Red, Green, Blue }
            let fluid x: Color = Color::Red;
            x = Color::Green;
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_block_stmt_ok() {
        let result = resolve_source("
        let mut y: int = 10;
        {
            y = 5;
            let mut x: int = 5;
            x = 6;
            enum C { C, Cpp }
            func foo() { let mut x: int = 10; x = 8; }
        }
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_if_stmt_ok() {
        let result = resolve_source("
        if ( 5 > 3 ) {
            let mut x: char = 'c';
            x = '5';
        }
        ");
        assert!(result.is_ok());

        let result = resolve_source("
        if ( 5 > 3 ) {
            let mut x: int = 5;
            x = 6;
        } else {
            enum C { C, Cpp }
            func foo() { let mut x: int = 10; x = 8; }
        }
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_while_stmt_ok() {
        let result = resolve_source("
            let mut x: int = 0;
            while ( x < 10 ) {
                print(x);
                x = x + 1;
            }
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_for_stmt_ok() {
        let result = resolve_source("
            for ( let mut i: int = 0; i < 10; i = i + 1; ) {
                print(\"Hello world!\");
            }
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_func_stmt_ok() {
        let result = resolve_source("
            func foo(a: int) {
                print(a + 1);
            }
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_return_stmt_ok() {
        let result = resolve_source("
            func foo(a: int) -> int {
                print(a + 1);
                return a + 2;
            }
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_struct_stmt_ok() {
        let result = resolve_source("
            struct Type {
                x: int,
                y: char,
                z: bool
            }
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_impl_stmt_ok() {
        let result = resolve_source("
            struct Type {
                x: int
            }

            impl Type {
                func set_x(a: int) {
                    return self.x = a;
                } 
            }
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_enum_stmt_ok() {
        let result = resolve_source("
            enum Color {
                Red,
                Green,
                Blue
            }
        ");
        assert!(result.is_ok());
    }

    // ! -- err tests --

    // ! -- exprs --

    #[test]
    fn test_resolver_call_expr_err() {
        let result = resolve_source("foo();");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Function not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }

        let result = resolve_source("p.x.b();");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Variable or method not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }

        let result = resolve_source("
            struct Type {
                x: int
            }
            impl Type {
                func foo() {}
            }
            Type::new();
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Method not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_resolver_variable_expr_err() {
        let result = resolve_source("x;");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Variable or method not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_resolver_structlit_expr_err() {
        let result = resolve_source("
            enum Color {
                Red,
                Green,
                Blue
            }
            let fluid x: Color = Color { };
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Expected struct, got enum.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }

        let result = resolve_source("
            struct Type {
                x: int
            }
            let mut x: Type = Type { x: 6, x: 5 };
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "This field is already declared.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }

        let result = resolve_source("
            let mut x: Type = Type { x: 5 };
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Type not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_parser_get_expr_err() {
        let result = resolve_source("p.x;");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Variable or method not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_parser_path_expr_err() {
        let result = resolve_source("
            enum Color {
                Red,
                Green,
                Blue,
            }
            Color::Purple;
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Variant not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }

        let result = resolve_source("
            struct Type {
                x: int
            }
            Type::x;
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "You can path only to methods with this type.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }

        let result = resolve_source("
            Type::new();
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Impl type not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    // ! -- stmts --

    #[test]
    fn test_resolver_let_declaration_stmt_err() {
        let result = resolve_source("let const x: int = x;");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Variable is used in self declarement.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_resolver_assign_stmt_err() {
        let result = resolve_source("x = 5;");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Variable not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }

        let result = resolve_source("let const x: int = 5; x = 10;");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Cannot assign to a const variable.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }

        let result = resolve_source("func foo(a: int) { a = 5; }");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Cannot assign function or its arguments.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_resolver_block_stmt_err() {
        let result = resolve_source("
            {
                let mut x: int = 5;
            }
            x = 10;
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Variable not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_resolver_for_stmt_err() {
        let result = resolve_source("
            while (let mut i: int; i < 3; i = i + 1) {
                print(i);
            }
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Variable or method not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_resolver_struct_stmt_err() {
        let result = resolve_source("
            struct Type {
                x: int
            }
            struct Type {
                y: float
            } 
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "This type is already declared.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }

        let result = resolve_source("
            struct Type {
                x: int,
                x: float
            }
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "This field is already declared.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_resolver_impl_stmt_err() {
        let result = resolve_source("
            impl Type {
                func new() {}
            }
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Type not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_resolver_enum_stmt_err() {
        let result = resolve_source("
            struct Type {
                x:int
            }
            enum Type {
                Something
            }
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "This type is already declared.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }

        let result = resolve_source("
            enum Color {
                Red, Red
            }
        ");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "This variant is already declared.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    // ! -- others --

    #[test]
    fn test_resolver_main_func_stmt_ok() {
        let result = resolve_source_with_main("func main() {}");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_main_func_stmt_err() {
        let result = resolve_source_with_main("func foo() {}");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "'main' function not found.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_resolver_top_level_ok() {
        let result = resolve_source_with_main("
            func foo() {}
            struct Struct { x: int }
            impl Struct { func new() {} }
            enum Color { Red, Green, Blue }
            func main() {}
        ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_resolver_top_level_err() {
        let result = resolve_source_with_main("x = 5;");
        match result {
            Err(e) => {
                match &e[0] {
                    HypercError::ResolveError { message, .. } => {
                        assert_eq!(message, "Only functions, structs, impls and enums are top-level-supported.")
                    }
                    _ => panic!("Expected ResolveError.")
                }
            }
            Ok(_) => panic!("Expected error.")
        }
    }
}