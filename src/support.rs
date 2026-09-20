use crate::token::{Token, TokenType};
use crate::ast::{Expr, Stmt, VarType};
use crate::checker::Type;
use std::ops::Range;

pub fn expr_span(expr: &Expr) -> Range<usize> {
    match expr {
        Expr::Binary    { operator, ..  } => operator.start..operator.end,
        Expr::Unary     { operator, ..  } => operator.start..operator.end,
        Expr::Call      { callee, ..} => expr_span(callee),
        Expr::Variable  { name          } => name.start..name.end,
        Expr::Literal   { span,..} => span.clone(),
        Expr::Grouping  { expr      } => expr_span(expr),
        Expr::StructLit { name, ..      } => name.start..name.end,
        Expr::Get       { item, ..      } => item.start..item.end,
        Expr::Path      { type_name, .. } => type_name.start..type_name.end,
        Expr::SelfExpr  { self_tok      } => self_tok.start..self_tok.end,
        _ => 0..0
    }
}

pub fn stmt_span(stmt: &Stmt) -> Range<usize> {
    match stmt {
        Stmt::Expression { value } => expr_span(value),
        Stmt::Print { value } => expr_span(value),
        Stmt::Let { name, .. } => name.start..name.end,
        Stmt::Assign { target, .. } => expr_span(target),
        Stmt::Block { statements } => match statements.last() {
            Some(stmt) => stmt_span(stmt),
            None => 0..0
        }
        Stmt::If { condition, .. } => expr_span(condition),
        Stmt::While { condition, .. } => expr_span(condition),
        Stmt::For { statements, .. } => stmt_span(statements),
        Stmt::Func { name, .. } => name.start..name.end,
        Stmt::Return { value, ret_tok } => match value {
            Some(expr) => expr_span(expr),
            None => ret_tok.start..ret_tok.end
        }
        Stmt::Struct { name, .. } => name.start..name.end,
        Stmt::Impl { name, .. } => name.start..name.end,
        Stmt::Enum { name, .. } => name.start..name.end,
    }
}

pub fn mangle(ty: &str, name: &str) -> String {
    format!("{},{}", ty, name)
}

pub fn var_type_to_type(vt: &VarType) -> Type {
    match vt {
        VarType::Int    => Type::Int,
        VarType::Float  => Type::Float,
        VarType::Str    => Type::Str,
        VarType::Char   => Type::Char,
        VarType::Bool   => Type::Bool,
        VarType::Named(tok) =>  Type::Named(tok.lexeme.clone()),
        VarType::Error  => Type::Error
    }
}

pub fn is_comparison(op: &Token) -> bool {
    match op.token_type {
        TokenType::Less |
        TokenType::LessEqual |
        TokenType::Greater |
        TokenType::GreaterEqual |
        TokenType::BangEqual |
        TokenType::EqualEqual => true,
        
        _ => false
    }
}