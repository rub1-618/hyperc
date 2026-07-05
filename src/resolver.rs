use crate::error::{ParseError};
use crate::token::{Token};
use crate::ast::{Expr, Stmt, VarKind, VarType};
use std::collections::HashMap;
use std::ops::Range;

#[derive(Debug, Clone)]
pub struct Binding {
    ready: bool,
    kind: VarKind, // todo
    var_type: VarType, // todo
}

// ! -- resolver --
#[derive(Debug, Clone)]
pub struct Resolver {
    scopes: Vec<HashMap<String, Binding>>,
}

impl Resolver {

    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    pub fn resolve(&mut self, statements: &[Stmt]) -> Result<(), ParseError>  {
        self.begin_scope();
        let mut has_main = false;
        for statement in statements {
            match statement {
                Stmt::Function { name, .. } => {
                    if &name.lexeme == "main" {
                        has_main = true;
                    }
                    self.resolve_stmt(statement)?;
                },
                Stmt::Class { .. } => {
                    self.resolve_stmt(statement)?;
                },
                _ => return Err(ParseError { 
                    span: Self::stmt_span(statement),
                    message: "Only functions and classes are top-level-supported.".to_string() 
                })
            }
        }
        if !has_main {
            return Err(ParseError { 
                    span: 0..0,
                    message: "'main' function not found.".to_string() 
                })
        }
        Ok(())
    }

    pub fn resolve_stmt(&mut self, statement: &Stmt) -> Result<(), ParseError> {
        match statement {

            Stmt::Expression { value } => {
                self.resolve_expr(value)
            },

            Stmt::Print { value } => {
                self.resolve_expr(value)
            },

            Stmt::Let { name, value , kind, var_type} => {
                self.declare(name, kind.clone(), var_type.clone())?;
                self.resolve_expr(value)?;
                self.define(name);
                Ok(())
            },

            Stmt::Assign { name, value } => {
                self.resolve_local(name)?;
                match self.lookup_binding(name) {
                    Some(b) => {
                        if b.kind == VarKind::Const {
                            return Err(ParseError { 
                                span: name.start..name.end,
                                 message: "Cannot redeclare const variable.".to_string() 
                            })
                        }
                    }
                    None => {}
                }
                self.resolve_expr(value)?;
                Ok(())
            },

            Stmt::Block { statements } => {
                self.resolve_stmts(statements)?;

                Ok(())
            },

            Stmt::If { params, then_branch, else_branch } => {
                self.resolve_expr(params)?;
                self.resolve_stmt(then_branch)?;
                if let Some(else_stmt) = else_branch {
                    self.resolve_stmt(else_stmt)?;
                }
                Ok(())
            },

            Stmt::While { conditions, statements } => {
                self.resolve_expr(conditions)?;
                self.resolve_stmt(statements)?;
                Ok(())
            }
            
            Stmt::For { initializer, condition, increment, statements } => {
                self.begin_scope();
                let result =( || {
                    if let Some(init) = initializer { self.resolve_stmt(init)?; }
                    if let Some(cond) = condition { self.resolve_expr(cond)?; }
                    if let Some(incr) = increment { self.resolve_stmt(incr)?; }
                    self.resolve_stmt(statements)?;
                    Ok(())
                })();
                self.end_scope();
                result
            },

            Stmt::Return { value } => {
                if let Some(return_expr) = value {
                    self.resolve_expr(return_expr)?;
                }
                Ok(())
            },

            Stmt::Function { name, params, statements, return_type: _ } => {
                self.fc_declare(name)?;
                self.define(name);
                self.begin_scope();
                let result = ( || {
                    for param in params {
                        self.fc_declare(&param.0)?;
                        self.define(&param.0);
                    }
                    self.resolve_stmt(statements)?;
                    Ok(())
                })();
                self.end_scope();
                result
            },

            Stmt::Class { name, superclass, methods } => {
                self.fc_declare(name)?;
                self.define(name);
                if let Some(sprclss) = superclass {
                    self.resolve_local(sprclss)?;
                }
                self.begin_scope();
                let result = ( || {
                    for method in methods {
                        self.resolve_stmt(method)?;
                    }
                    Ok(())
                })();
                self.end_scope();
                result
            },
        }
    }

    pub fn resolve_expr(&mut self, expression: &Expr) -> Result<(), ParseError> {
        match expression {
            
            Expr::Variable {name} => { 
                for scope in self.scopes.iter().rev() {
                    match scope.get(&name.lexeme) {
                        Some(binding) => {
                            if !binding.ready {
                                return Err( ParseError { 
                                    span: name.start..name.end, 
                                    message: "Variable is used in self-declarement.".to_string(),
                                });
                            }
                            return Ok(());
                        }
                        None => {}
                    }
                }
                Err( ParseError { 
                    span: name.start..name.end, 
                    message: "Variable not found.".to_string(),
                })
                }
            _ => Ok(())
        }
    }
    
    fn resolve_local(&mut self, name: &Token) -> Result<(), ParseError> {
        for scope in self.scopes.iter().rev() {
            match scope.get(&name.lexeme) {
                Some(binding) => {
                    if !binding.ready {
                        return Err( ParseError { 
                            span: name.start..name.end, 
                            message: "Variable is used in self-declarement.".to_string(),
                        });
                    }
                    return Ok(());
                }
                None => {} 
            }
        }
        Err( ParseError { 
            span: name.start..name.end, 
            message: "Variable not found.".to_string(),
        })
    }

    fn declare(&mut self, name: &Token, kind: VarKind, var_type: VarType) -> Result<(), ParseError> {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(&name.lexeme.clone()) {
                return Err( ParseError { 
                    span: name.start..name.end, 
                    message: "Variable is already declared.".to_string(),
                });
            }
            scope.insert(name.lexeme.clone(), Binding { ready: false, kind, var_type } );
        }
        Ok(())
    }

    fn fc_declare(&mut self, name: &Token) -> Result<(), ParseError> {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(&name.lexeme.clone()) {
                return Err( ParseError { 
                    span: name.start..name.end, 
                    message: "Already declared.".to_string(),
                });
            }
            scope.insert(name.lexeme.clone(), Binding { ready: false, kind: VarKind::Mut, var_type: VarType::Str, } );
        }                                                                       // kind and type are simple plugs
        Ok(())
    }

    fn lookup_binding(&self, name: &Token) -> Option<&Binding> {
        for scope in self.scopes.iter().rev() {
            if let Some(b) = scope.get(&name.lexeme) {
                return Some(b)
            }
        }
        None
    }

    fn stmt_span(stmt: &Stmt) -> Range<usize> {
        match stmt {
            Stmt::Let { name, .. } => {name.start..name.end},
            Stmt::Assign { name, .. } => {name.start..name.end},
            Stmt::Expression { value } => {Self::expr_span(value)},
            _ => {0..0},
        }
    }

    fn expr_span(expr: &Expr) -> Range<usize> {
        match expr {
            Expr::Binary { operator, .. } => operator.start..operator.end,
            Expr::Unary { operator, .. } => operator.start..operator.end,
            Expr::Call { paren, .. } => paren.start..paren.end,
            Expr::Variable { name } => name.start..name.end,
            Expr::Literal { span, .. } => span.clone(),
            Expr::Grouping { expr } => Self::expr_span(expr),
        }
    }

    fn define(&mut self, name: &Token) {
        if let Some(scope) = self.scopes.last_mut() {
            if let Some(binding) = scope.get_mut(&name.lexeme) {
                binding.ready = true;
            }
        }
    }

    fn begin_scope(&mut self) {
        self.scopes.push( HashMap::new() );
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    
    fn resolve_stmts(&mut self, stmts: &[Stmt]) -> Result<(), ParseError> {
        self.begin_scope();
        for stmt in stmts {
            self.resolve_stmt(stmt)?;
        }
        self.end_scope();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::lexer::{Lexer};
    use crate::parser::{Parser};
    use crate::resolver;

    #[test]
    fn test_resolve_ok() {
        let mut lexer = Lexer::new("let mut x: int = 5;".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        assert!(_resolver.resolve_stmts(&stmts).is_ok() );
    }

    #[test]
    fn test_undefined() {
        let mut lexer = Lexer::new("let mut y: int = z;".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        let result = _resolver.resolve_stmts(&stmts);
        match result {
            Err (e) => assert!(e.message.contains("Variable not found.")),
            Ok (()) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_self_ref() {
        let mut lexer = Lexer::new("let mut z: int = z;".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        let result = _resolver.resolve_stmts(&stmts);
        match result {
            Err (e) => assert!(e.message.contains("Variable is used in self-declarement.")),
            Ok (()) => panic!("Expected error.")
        }
    }
    
    #[test]
    fn test_redeclare() {
        let mut lexer = Lexer::new("let mut x: int = 5; let mut x: int = 10;".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        let result = _resolver.resolve_stmts(&stmts);
        match result {
            Err (e) => assert!(e.message.contains("Variable is already declared.")),
            Ok (()) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_block() {
        let mut lexer = Lexer::new("let mut x: int = 5; { let mut y: int = x; }".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        assert!(_resolver.resolve_stmts(&stmts).is_ok() );
    }

    #[test]
    fn test_for_leak() {
        let mut lexer = Lexer::new("for(let mut i: int = 0; i<5; i = i + 1){} let mut z: int = i;".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        let result = _resolver.resolve_stmts(&stmts);
        match result {
            Err (e) => assert!(e.message.contains("Variable not found.")),
            Ok (()) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_for_inner() {
        let mut lexer = Lexer::new("let mut x: int = 5; for(let mut i: int = 0; i<x; i = i + 2){}".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        assert!(_resolver.resolve_stmts(&stmts).is_ok() );
    }

    #[test]
    fn test_parameter_leak() {
        let mut lexer = Lexer::new("fn foo(i: int){} let mut z: int = i;".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        let result = _resolver.resolve_stmts(&stmts);
        match result {
            Err (e) => assert!(e.message.contains("Variable not found.")),
            Ok (()) => panic!("Expected error.")
        }
    }
    
    #[test]
    fn test_fn_leak() {
        let mut lexer = Lexer::new("fn foo(i: char){ let mut b: int = i; }".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        assert!(_resolver.resolve_stmts(&stmts).is_ok() );
    }

    #[test]
    fn test_fn_double() {
        let mut lexer = Lexer::new("fn foo(i: int, i:int){}".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        let result = _resolver.resolve_stmts(&stmts);
        match result {
            Err (e) => assert!(e.message.contains("Already declared.")),
            Ok (()) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_class() {
        let mut lexer = Lexer::new("class C { fn foo(){ let mut q: int = 1; } }".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        assert!(_resolver.resolve_stmts(&stmts).is_ok() );
    } 

    #[test]
    fn test_block_self_ref() {
        let mut lexer = Lexer::new("{ let const i: char = i; }".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        let result = _resolver.resolve_stmts(&stmts);
        match result {
            Err (e) => assert!(e.message.contains("Variable is used in self-declarement.")),
            Ok (()) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_const_assign_err() {
        let mut lexer = Lexer::new("let const x: bool = true; x = false;".to_string());
        let _tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(_tokens.clone());
        let stmts = _parser.parse().unwrap();
        let mut _resolver = resolver::Resolver::new();
        assert!(_resolver.resolve_stmts(&stmts).is_err() );
    }
}