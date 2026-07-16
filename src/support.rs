use crate::token::{Token, TokenType};
use crate::ast::{Expr, Stmt, VarType};
use std::ops::Range;
use crate::checker::Type;

pub fn mangle(ty: &str, name: &str) -> String {
        format!("{}.{}", ty, name)
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
    
pub fn expr_span(expr: &Expr) -> Range<usize> {
    match expr {
       Expr::Binary { operator, .. } => operator.start..operator.end,
       Expr::Unary { operator, .. } => operator.start..operator.end,
       Expr::Call { paren, .. } => paren.start..paren.end,
       Expr::Variable { name } => name.start..name.end,
       Expr::Literal { span, .. } => span.clone(),
       Expr::Grouping { expr } => expr_span(expr),
       Expr::StructLit { name, .. } => name.start..name.end,
       Expr::Get {  field, .. } => field.start..field.end,
       Expr::Path { type_name, .. } => type_name.start..type_name.end,
       Expr::SelfExpr { self_tok } => self_tok.start..self_tok.end,
    }
}

pub fn stmt_span(stmt: &Stmt) -> Range<usize> {
    match stmt {
        Stmt::Expression { value } => expr_span(value),
        Stmt::Print { value } => expr_span(value),
        Stmt::Let { name, .. } => name.start..name.end,
        Stmt::Assign { target, .. } => expr_span(target),
        Stmt::Block { statements } => {
            match statements.last() {
                Some(last) => stmt_span(last), // returns span of the last stmt in the block
                None => 0..0 // or a zero | no span source
            }
        }
        Stmt::If { params, .. } => expr_span(params),
        Stmt::While { conditions, .. } => expr_span(conditions),
        Stmt::For { statements, .. } => stmt_span(statements),
        Stmt::Return { value } => {
            match value {
                Some(val) => expr_span(val),
                None => 0..0 // in future will be fixed when tokens for each stmt are 
                             // added like in SelfExpr | v0.5+ | no span source
            }
        }
        Stmt::Function { name, .. } => name.start..name.end,
        Stmt::Struct { name, .. } => name.start..name.end,
        Stmt::Impl { name, .. } => name.start..name.end,
        Stmt::Enum { name, .. } => name.start..name.end,
    }
}

 pub fn is_comparison(op: &Token) -> bool {
    match op.token_type {
        TokenType::Less |
        TokenType::LessEqual |
        TokenType::Greater |
        TokenType::GreaterEqual |
        TokenType::BangEqual |
        TokenType::EqualEqual => {true}
        
        _ => false
    }
}