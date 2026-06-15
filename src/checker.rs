use std::collections::HashMap;
use crate::token::{TokenType, Token};
use crate::ast::{Expr, Stmt, LiteralValue, VarKind, VarType};
use crate::error::TypeError;

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum Type {
    Int,
    Float,
    Str,
    Char,
    Bool,
    Unit,
    Error,
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
                     TokenType::Star | TokenType::Slash |
                      TokenType::Percent => {
                        if (lt == Type::Int || lt == Type::Float) && (rt == Type::Int || rt == Type::Float) {
                             if lt == Type::Float || rt == Type::Float {
                                Ok(Type::Float)
                             } else {
                                Ok(Type::Int)
                             }
                        } else {
                            return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: "Arithmetic operands must be numeric.".to_string() 
                            })
                        }
                      },

                    TokenType::Less | TokenType::LessEqual |
                     TokenType::Greater | TokenType::GreaterEqual => {
                        if (lt == Type::Int || lt == Type::Float) && (rt == Type::Int || rt == Type::Float) {
                            Ok(Type::Bool)
                        } else {
                            return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: "Invalid comparison operands.".to_string() 
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
                                message: "Invalid comparison operands.".to_string() 
                            })
                        }
                    },

                    TokenType::And | TokenType::Or => {
                        if lt == Type::Bool && rt == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: "Logical operands must be booleans.".to_string() 
                            })
                        }
                    },

                    _ => {
                        return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: "Unsupported binary operator.".to_string() 
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
                                message: "Unsupported unary operand.".to_string() 
                            })
                        }
                    },

                    TokenType::Bang => {
                        if right == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: "For '!' operator only boolean operadns are supported.".to_string() 
                            })
                        }
                    },

                    _ => {
                        return Err(TypeError { 
                                span: operator.start..operator.end, 
                                message: "Unsupported unary operator.".to_string() 
                            })
                        }
                }
            }

            Expr::Call { callee, arguments, paren } => {
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
                    
            

            Expr::Literal { value } => {
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

            _ => todo!()
        }
    }

    pub fn vartype_to_type(vt: &VarType) -> Type {
        match vt {
            VarType::Int => {Type::Int},
            VarType::Float => {Type::Float},
            VarType::Str => {Type::Str},
            VarType::Char => {Type::Char},
            VarType::Bool => {Type::Bool},
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
                                message: "Mismatched type.".to_string() 
                            })
                }
            }

            Stmt::Assign { name, value } => {
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
                }

                return Err(TypeError { 
                                span: name.start..name.end, 
                                message: "Mismatched type.".to_string() 
                            })
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
                let params = self.infer(params)?;
                if params_expected == params {
                    self.check_stmt(then_branch)?;
                    if let Some(else_stmt) = else_branch {
                        self.check_stmt(else_stmt)?;
                    }
                } else {
                    return Err(TypeError { 
                                span: 0..0, // todo: span for statements
                                message: "Invalid if statement.".to_string() 
                            })
                        }

                Ok(())
            }

            Stmt::While { conditions, statements } => {
                let cond_expected = Type::Bool;
                let condition = self.infer(conditions)?;
                if cond_expected == condition {
                    self.check_stmt(statements)?;
                } else {
                    return Err(TypeError { 
                                span: 0..0, // todo: span for statements
                                message: "Invalid while statement.".to_string() 
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
                        let ct = self.infer(cond)?;
                        if ct != cond_expected {
                            return Err(TypeError { 
                                span: 0..0, // todo: span for statements
                                message: "For conditions must be boolean.".to_string() 
                            })
                        }
                    }
                    if let Some(incr) = increment {
                        self.infer(incr)?; 
                    }
                    self.check_stmt(statements)?;
                    Ok(())
                })();
                self.end_scope();
                result
            }

            Stmt::Return { value } => {
                    let ret_type = match value {
                        Some(v) => self.infer(v)?,
                        None => Type::Unit,
                    };

                    match self.current_return {
                        Some(expected) => {
                            if expected == ret_type {
                                return Ok(())
                            } else {
                                return Err(TypeError { 
                                    span: 0..0,
                                    message: "Mismatched return type.".to_string() 
                                })
                            }
                        }
                        None => return Err(TypeError { 
                                span: 0..0,
                                message: "Return should be inside the function.".to_string() 
                            })
                        }
            }

            Stmt::Function { name, params, statements , return_type} => {
                let prev = self.current_return;
                let prev_fn = match return_type{
                    Some(vt) => Self::vartype_to_type(vt),
                    None => Type::Unit,
                };
                self.current_return = Some(prev_fn);
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

            _ => Ok(())
        }
    }

    fn declare(&mut self, name: &Token, ty: Type) -> Result<(), TypeError> {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.lexeme.clone(), ty );
        }
        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(*ty)
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

    #[test]
    fn test_int() {
        assert_eq!(infer_source("1 + 2;").unwrap(), Type::Int )
    }

    #[test]
    fn test_float() {
        assert_eq!(infer_source("1.0 + 2;").unwrap(), Type::Float)
    }

    #[test]
    fn test_err() {
        assert!(infer_source("\"1.0\" + 2;").is_err())
    }

    #[test]
    fn test_source_int_ok() {
        assert!(check_source("let mut x: int = 1 + 1;").is_ok())
    }

    #[test]
    fn test_source_err() {
        assert!(check_source("let mut x: int = \"hello\";").is_err())
    }

    #[test]
    fn test_source_bool_ok() {
        assert!(check_source("let const x: bool = true;").is_ok())
    }
}