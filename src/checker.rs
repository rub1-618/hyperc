use std::ops::Range;
use ariadne::{ Report, ReportKind, Label, Source };
use crate::token::{TokenType, Token};
use crate::ast::{Expr, Stmt, LiteralValue, VarKind, VarType};
use crate::lexer::Lexer;
use crate::parser::Parser;

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
pub struct TypeError {
     pub span:  Range<usize>,
     pub message: String,
}

pub struct TypeChecker {}

impl TypeChecker {
    pub fn new() -> Self {
        Self {}
    }

    pub fn infer(&mut self, expr: &Expr) -> Result<Type, TypeError> {
        match expr {
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
                                message: "Arithmetic operands must be numeric.".to_string() })
                        }
                      }
                    _ => todo!()
                }
            },

            Expr::Grouping { expr } => self.infer(expr),

            _ => todo!()
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn infer_source(src: &str) -> Result<Type, TypeError> {
        let mut lexer = Lexer::new(src  .to_string());
        let tokens = lexer.scan_tokens();
        let mut _parser = Parser::new(tokens.clone());        
        let stmt = _parser.parse().unwrap();
        let mut _infer = TypeChecker::new();
        if let Stmt::Expression { value } = &stmt[0] {
            _infer.infer(value)
        } else { panic!("Expected expression statement."); }
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
}