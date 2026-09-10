// use crate::token::{Token, TokenType};
use crate::ast::{Expr};
use std::ops::Range;

pub fn expr_span(expr: &Expr) -> Range<usize> {
    match expr {
        Expr::Binary    { operator, ..  } => operator.start..operator.end,
        Expr::Unary     { operator, ..  } => operator.start..operator.end,
        Expr::Call      { paren, ..     } => paren.start..paren.end,
        Expr::Variable  { name          } => name.start..name.end,
        Expr::Literal   { span,..} => span.clone(),
        Expr::Grouping  { expr      } => expr_span(expr),
        Expr::StructLit { name, ..      } => name.start..name.end,
        Expr::Get       { field, ..     } => field.start..field.end,
        Expr::Path      { type_name, .. } => type_name.start..type_name.end,
        Expr::SelfExpr  { self_tok      } => self_tok.start..self_tok.end,
        _ => 0..0
    }
}