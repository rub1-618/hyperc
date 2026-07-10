use std::collections::HashMap;
use std::ops::Range;
use crate::token::{TokenType, Token};
use crate::ast::{Expr, Stmt, LiteralValue, VarType};
use crate::error::TypeError;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Str,
    Char,
    Bool,
    Unit,
    Named(String),
    Error, // todo
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Str => write!(f, "str"),
            Type::Char => write!(f, "char"),
            Type::Bool => write!(f, "bool"),
            Type::Unit => write!(f, "()"),
            Type::Named(name) => write!(f, "{}", name),
            Type::Error => write!(f, "<error>"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FnSig {
    params: Vec<Type>,
    ret: Type,
}

pub struct TypeChecker {
    scopes: Vec<HashMap<String, Type>>,
    current_return: Option<Type>,
    functions: HashMap<String, FnSig>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self { scopes: Vec::new(), current_return: None, functions: HashMap::new() }
    }
    
    pub fn check(&mut self, statements: &[Stmt]) -> Result<(), TypeError> {
        self.begin_scope();
        for statement in statements {
            self.check_stmt(statement)?;    
        }
        self.end_scope();
        Ok(())
    }

    pub fn infer(&mut self, expr: &Expr) -> Result<Type, TypeError> {
        match expr {

            Expr::Binary { left, operator, right } => {
                let lt = self.infer(left)?;
                let rt = self.infer(right)?;
                match operator.token_type {
                    TokenType::Plus | TokenType::Minus |
                     TokenType::Star | TokenType::Slash => {
                        if (lt == Type::Int || lt == Type::Float) && (rt == Type::Int || rt == Type::Float) {
                             if lt == Type::Float || rt == Type::Float {
                                Ok(Type::Float)
                             } else {
                                Ok(Type::Int)
                             }
                        } else {
                            return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: format!("Arithmetic operands must be numeric, got {} and {}.", lt, rt)
                            })
                        }
                      },

                    TokenType::Percent => {
                        if lt == Type::Int && rt == Type::Int {
                            Ok(Type::Int)
                        } else {
                            return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: format!("Modulo operands must be both int, got {} and {}.", lt, rt)
                            })
                        }
                    }

                    TokenType::Less | TokenType::LessEqual |
                     TokenType::Greater | TokenType::GreaterEqual => {
                        if (lt == Type::Int || lt == Type::Float) && (rt == Type::Int || rt == Type::Float) {
                            Ok(Type::Bool)
                        } else {
                            return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: format!("Comparison operands must be numeric, got {} and {}.", lt, rt) 
                            })
                        }
                      },

                    TokenType::EqualEqual | TokenType::BangEqual => {
                        if (lt == Type::Int || lt == Type::Float) && (rt == Type::Int || rt == Type::Float) {
                            Ok(Type::Bool)
                        } else if lt == rt {
                            Ok(Type::Bool)
                        } else {
                            return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: format!("Comparison operands must be numeric, got {} and {}.", lt, rt) 
                            })
                        }
                    },

                    TokenType::And | TokenType::Or => {
                        if lt == Type::Bool && rt == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: format!("Logical operands must be both booleans, got {} and {}.", lt, rt) 
                            })
                        }
                    },

                    _ => {
                        return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: "This binary operator is not supported in v0.1.".to_string() 
                            })
                    },
                }
            },

            Expr::Unary { operator, right } => {
                let right = self.infer(right)?;
                match operator.token_type {
                    TokenType::Minus => {
                        if right == Type::Int {
                            Ok(Type::Int)
                        } else if right == Type::Float {
                            Ok(Type::Float)
                        } else {
                            return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: format!("Expected mathematical operand for '-', got {}.", right)
                            })
                        }
                    },

                    TokenType::Bang => {
                        if right == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: format!("Expected boolean operand for '!', got {}.", right) 
                            })
                        }
                    },

                    _ => {
                        return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: "This unary operator is not supported in v0.1.".to_string() 
                            })
                        }
                }
            }

            Expr::Call { callee, arguments, paren: _ } => {
                match callee.as_ref() {
                    Expr::Variable { name} => {
                        
                        let sig = match self.functions.get(&name.lexeme) {
                            Some(s) => s.clone(),
                            None => return Err(TypeError { 
                                span: name.start..name.end, 
                                message: "Unknown function call.".to_string() 
                            }),
                        };

                        if !(sig.params.len() == arguments.len()) {
                            return Err(TypeError { 
                                span: name.start..name.end, 
                                message: "Wrong number of arguments.".to_string() 
                            });
                        }

                        for i in 0..arguments.len() {
                            let arg_ty = self.infer(&arguments[i])?;
                            if arg_ty != sig.params[i] {
                                return Err(TypeError { 
                                    span: name.start..name.end, 
                                    message: "Argument type mismatched.".to_string() 
                                });
                            }
                        }

                        Ok(sig.ret) 
                    }

                    _ => Err(TypeError { 
                        span: 0..0, 
                        message: "Expression is not callable.".to_string(),
                    })
                }
            }

            Expr::Literal { value, .. } => {
                match value {
                    LiteralValue::Int(_) => Ok(Type::Int),
                    LiteralValue::Float(_) => Ok(Type::Float),
                    LiteralValue::String(_) => Ok(Type::Str),
                    LiteralValue::Char(_) => Ok(Type::Char),
                    LiteralValue::Bool(_) => Ok(Type::Bool),
                    LiteralValue::Null => Ok(Type::Unit),
                }
            }

            Expr::Grouping { expr } => self.infer(expr),

            Expr::Variable { name } => {
                match self.lookup(&name.lexeme) {
                    Some(ty) => Ok(ty),
                    _ => {
                        return Err(TypeError { 
                                span: name.start..name.end, 
                                message: "Cannot define variable type.".to_string() 
                            })
                        }
                }
            }

            Expr::StructLit { name, .. } => {
                return Err(TypeError { 
                    span: name.start..name.end, 
                    message: "Struct literals are not supported yet.".to_string() 
                });
            }

            Expr::Get {  field, .. } => {
                return Err(TypeError { 
                    span: field.start..field.end, 
                    message: "Field access is not supported yet.".to_string()
                })
            }

            Expr::Path { type_name, .. } => {
                return Err(TypeError { 
                    span: type_name.start..type_name.end, 
                    message: "Paths are not supported yet.".to_string()
                })
            }
        }
    }
      
    pub fn vartype_to_type(vt: &VarType) -> Type {
        match vt {
            VarType::Int => {Type::Int},
            VarType::Float => {Type::Float},
            VarType::Str => {Type::Str},
            VarType::Char => {Type::Char},
            VarType::Bool => {Type::Bool},
            VarType::Named(tok) => Type::Named(tok.lexeme.clone()),
        }
    }

    pub fn check_stmt( &mut self, stmt: &Stmt ) -> Result<(), TypeError> {
        match stmt {

            Stmt::Expression { value } => {self.infer(value)?; Ok(())}

            Stmt::Print { value } => {self.infer(value)?; Ok(())}

            Stmt::Let { name, value, kind: _, var_type } => {
                let ty = self.infer(value)?;
                let expected = Self::vartype_to_type(var_type);
                if ty == expected {
                    self.declare(name, ty)
                } else {
                    return Err(TypeError { 
                                span: name.start..name.end, 
                                message: format!("Mismatched type. Got: {}, expected: {}.", ty, expected)
                            })
                }
            }

            Stmt::Assign {  target, value } => {
                match  &**target {
                    Expr::Variable { name } => {
                        let expected = match self.lookup(&name.lexeme) {
                            Some(t) => t,
                            _ => return Err(TypeError { 
                                    span: name.start..name.end, 
                                    message: "Unknown variable.".to_string() 
                                }),
                        };
                        let ty = self.infer(value)?;
                        if expected == ty {
                            return Ok(())
                        } else {
                            return Err(TypeError { 
                                span: name.start..name.end, 
                                message: format!("Mismatched type. Got: {}, expected: {}.", ty, expected)
                            })
                        }
                    }

                    Expr::Get { object, field } => {
                        return Err(TypeError { 
                            span: field.start..field.end, 
                            message: "Field assignment is not supported yet.".to_string()
                        });
                    }

                    _ => unreachable!()
                } 
            }

            Stmt::Block { statements } => {
                self.begin_scope();
                let result = ( || {
                    for statement in statements {
                        self.check_stmt(statement)?;
                    }
                    Ok(())
                })();
                self.end_scope();
                result
            }

            Stmt::If { params, then_branch, else_branch } => {
                let params_expected = Type::Bool;
                let span = Self::expr_span(&params);
                let params = self.infer(params)?;
                if params_expected == params {
                    self.check_stmt(then_branch)?;
                    if let Some(else_stmt) = else_branch {
                        self.check_stmt(else_stmt)?;
                    }
                } else {
                    return Err(TypeError { 
                                span,
                                message: format!("If condition must be boolean, got {}.", params), 
                            })
                        }

                Ok(())
            }

            Stmt::While { conditions, statements } => {
                let cond_expected = Type::Bool;
                let span = Self::expr_span(&conditions);
                let condition = self.infer(conditions)?;
                if cond_expected == condition {
                    self.check_stmt(statements)?;
                } else {
                    return Err(TypeError { 
                                span,
                                message: format!("While condition must be boolean, got {}.", condition) 
                            })
                        }

                Ok(())
            }

            Stmt::For { initializer, condition, increment, statements } => {
                self.begin_scope();
                let result = ( || {
                    if let Some(init) = initializer { self.check_stmt(init)?; }
                    let cond_expected = Type::Bool;
                    if let Some(cond) = condition {
                        let span = Self::expr_span(&cond);
                        let ct = self.infer(cond)?;
                        if ct != cond_expected {
                            return Err(TypeError { 
                                span,
                                message: format!("Invalid for condition. Got: {}, expected: {}.", ct, cond_expected)
                            })
                        }
                    }
                    if let Some(incr) = increment {
                        self.check_stmt(incr)?; 
                    }
                    self.check_stmt(statements)?;
                    Ok(())
                })();
                self.end_scope();
                result
            }

            Stmt::Return { value } => {
                let span = match value {
                    Some(sp) => Self::expr_span(sp),
                    None => 0..0, // todo
                };

                let ret_type = match value {
                    Some(v) => {self.infer(v)?},
                    None => Type::Unit,
                };

                match &self.current_return {
                    Some(expected) => {
                        if expected == &ret_type {
                            return Ok(())
                        } else {
                            return Err(TypeError { 
                                span,
                                message: format!("Mismatched return type. Got: {}, expected: {}.", ret_type, expected)
                            })
                        }
                    }
                    None => return Err(TypeError { 
                            span,
                            message: "Return should be inside the function.".to_string() 
                        })
                }
            }

            Stmt::Function { name, params, statements , return_type} => {
                let prev = self.current_return.take();
                let prev_fn = match return_type{
                    Some(vt) => Self::vartype_to_type(vt),
                    None => Type::Unit,
                };
                self.current_return = Some(prev_fn.clone());
                let mut param_types = vec![];
                for p in params {
                    param_types.push(Self::vartype_to_type(&p.1));
                }

                self.functions.insert( name.lexeme.clone(), FnSig { params: param_types, ret: prev_fn });

                self.begin_scope();
                let result = ( || {
                    for param in params {
                        self.declare(&param.0, Self::vartype_to_type(&param.1))?;
                    }
                    self.check_stmt(statements)?;
                    match return_type {
                        Some(_) => {
                            if !Self::always_return(statements) {
                                return Err(TypeError { 
                                    span: name.start..name.end,
                                    message: "Not all paths return.".to_string()
                                })
                            }
                        }
                        None => ()
                    }

                    Ok(())
                } )(); 

                self.end_scope();
                self.current_return = prev;
                result
            }

            Stmt::Class {  methods, .. } => {
                for method in methods {
                    self.check_stmt(method)?;
                }
                Ok(())
            }

            Stmt::Struct { name, fields } => {
                Ok(())
            }

            Stmt::Impl { name, methods } => {
                for method in methods {
                    self.check_stmt(method)?;
                }
                Ok(())
            }

            Stmt::Enum { name, variants } => {
                Ok(())
            }
        }
    }

    fn declare(&mut self, name: &Token, ty: Type) -> Result<(), TypeError> {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.lexeme.clone(), ty );
        }
        Ok(())
    }

    fn always_return(stmt: &Stmt) -> bool {
        match stmt {
            
            Stmt::Return { .. } => {
                true
            }

            Stmt::Block { statements } => {
                let check = statements.iter().any(|s: &Stmt| Self::always_return(s));
                if check {return true;} else {return false;}
            }

            Stmt::If { then_branch, else_branch , ..} => {
                match else_branch {
                    Some(b) => {
                        if Self::always_return(then_branch) && Self::always_return(b) {
                            true
                        } else { false }
                    }
                    None => {false}
                }
            }

            _ => false
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
            Expr::StructLit { name, .. } => name.start..name.end,
            Expr::Get { field, .. } => field.start..field.end,
            Expr::Path { type_name, .. } => type_name.start..type_name.end,
        }
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone())
            }
        }
        None
    }

    fn begin_scope(&mut self) {
        self.scopes.push( HashMap::new() );
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn infer_source(src: &str) -> Result<Type, TypeError> {
        let mut lexer = Lexer::new(src  .to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());        
        let stmt = _parser.parse().unwrap();
        let mut _infer = TypeChecker::new();
        if let Stmt::Expression { value } = &stmt[0] {
            _infer.infer(value)
        } else { panic!("Expected expression statement."); }
    }

    fn check_source(src: &str) -> Result<(), TypeError> {
        let mut lexer = Lexer::new(src  .to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());        
        let stmt = _parser.parse().unwrap();
        let mut _infer = TypeChecker::new();
        _infer.check_stmt( &stmt[0])
    }

    fn check_all_source(src: &str) -> Result<(), TypeError> {
        let mut lexer = Lexer::new(src  .to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());        
        let stmt = _parser.parse().unwrap();
        let mut _infer = TypeChecker::new();
        _infer.check(&stmt)?;
        Ok(())
    }

    #[test]
    fn test_int() {
        assert_eq!(infer_source("1 + 2;").unwrap(), Type::Int )
    }

    #[test]
    fn test_unar_int() {
        assert_eq!(infer_source("-4;").unwrap(), Type::Int )
    }

    #[test]
    fn test_float() {
        assert_eq!(infer_source("1.0 + 2;").unwrap(), Type::Float)
    }

    #[test]
    fn test_unar_bool() {
        assert_eq!(infer_source("!true;").unwrap(), Type::Bool)
    }

    #[test]
    fn test_unar_bool_err() {
        assert!(infer_source("-true;").is_err())
    }

    #[test]
    fn test_unar_int_err() {
        assert!(infer_source("!5;").is_err())
    }

    #[test]
    fn test_unar_str_err() {
        assert!(infer_source("-\"string\";").is_err())
    }

    #[test]
    fn test_comparison_less() {
        assert_eq!(infer_source("1 < 2;").unwrap(), Type::Bool)
    }

    #[test]
    fn test_comparison_greater() {
        assert_eq!(infer_source("1 > 2;").unwrap(), Type::Bool)
    }

    #[test]
    fn test_equality() {
        assert_eq!(infer_source("2 == 2;").unwrap(), Type::Bool)
    }

    #[test]
    fn test_bang_equality() {
        assert_eq!(infer_source("1 != 2;").unwrap(), Type::Bool)
    }

    #[test]
    fn test_err() {
        assert!(infer_source("\"1.0\" + 2;").is_err())
    }

    #[test]
    fn test_compar_err() {
        assert!(infer_source("\"a\" > 2;").is_err())
    }

    #[test]
    fn test_logicor() {
        assert_eq!(infer_source("true || false;").unwrap(), Type::Bool)
    }

    #[test]
    fn test_logicand() {
        assert_eq!(infer_source("true && false;").unwrap(), Type::Bool)
    }

    #[test]
    fn test_logic_err() {
        assert!(infer_source("\"a\" && true;").is_err())
    }

    #[test]
    fn test_source_int_ok() {
        assert!(check_source("let mut x: int = 1 + 1;").is_ok())
    }

    #[test]
    fn test_decl_err() {
        assert!(check_source("let mut x: int = \"hello\";").is_err())
    }

    #[test]
    fn test_bool_decl() {
        assert!(check_source("let const x: bool = true;").is_ok())
    }

    #[test]
    fn test_if_cond() {
        assert!(check_source("if (true) {}").is_ok())
    }

    #[test]
    fn test_if_cond_err() {
        assert!(check_source("if (5) {}").is_err())
    }

    #[test]
    fn test_while_cond() {
        assert!(check_source("while (true) {}").is_ok())
    }

    #[test]
    fn test_while_cond_err() {
        assert!(check_source("while (5) {}").is_err())
    }

    #[test]
    fn test_decl_assign() {
        assert!(check_all_source("let mut x: bool = true; x = false;").is_ok())
    }

    #[test]
    fn test_fn_ret() {
        assert!(check_source("fn foo() -> int { return 5; }").is_ok())
    }

    #[test]
    fn test_fn_ret_err() {
        assert!(check_source("fn foo() -> int { return \"5\"; }").is_err())
    }

    #[test]
    fn test_class() {
        assert!(check_source("class Dog {}").is_ok())
    }

    #[test]
    fn test_class_err() {
        assert!(check_source("class Dog { fn foo () -> int { return \"x\"; } }").is_err())
    }


    #[test]
    fn test_call_arg_num_err() {
        assert!(check_all_source("fn foo (x: int, y: int, z: bool) { return x; } foo(5, 7); ").is_err())
    }

    #[test]
    fn test_block_in_block() {
        assert!(check_all_source(" { let mut x: int = 3; { let const y: int = x; } } ").is_ok())
    }

    #[test]
    fn test_block_env_err() {
        assert!(check_all_source(" { let mut x: int = 3; } let const y: int = x; ").is_err())
    }

    #[test]
    fn test_range() {
        let result = check_source("if (5) { let mut x: int = 1; }");
        match result {
            Err(e) => {
                assert!(e.span != (0..0));
                assert!(e.span.start > 0);
                assert!(e.span.end > 0);
            }
            Ok(()) => panic!("Expected error")
        }
    }
}