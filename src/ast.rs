use crate::token::Token;
use std::{ops::Range, string::String};

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Binary  {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    }, 

    Unary {
        operator: Token,
        right: Box<Expr>,
    },

    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
        paren: Token,
    },

    Literal {
        value: LiteralValue,
        span: Range<usize>
    },

    Grouping {
        expr: Box<Expr>
    },

    Variable {
        name: Token,
    },

    StructLit {
        name: Token,
        fields: Vec<(Token, Box<Expr>)>,
    },

    Get {
        object: Box<Expr>,
        field: Token,
    },

    Path {
        type_name: Token,
        item: Token,
    },

    SelfExpr {
        self_tok: Token,
    },

    ErrorExpr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expression {
        value: Box<Expr>,
    },

    Print {
        value: Box<Expr>,
    },

    Let {
        name: Token,
        value: Box<Expr>,
        var_kind: VarKind,
        var_type: VarType,
    },

    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },

    Block {
        statements: Vec<Stmt>,
    },

    If {
        condition: Box<Expr>,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },

    While {
        condition: Box<Expr>,
        statements: Box<Stmt>,
    },

    For {
        initializer: Option<Box<Stmt>>,
        condition: Option<Box<Expr>>,
        increment: Option<Box<Stmt>>,
        statements: Box<Stmt>,
    },

    Return {
        value: Option<Box<Expr>>,
    },

    Func {
        name: Token,
        args: Vec<(Token, VarType)>,
        statements: Box<Stmt>,
        return_type: Option<VarType>,
    },

    Struct {
        name: Token,
        fields: Vec<(Token, VarType)>,
    },

    Impl {
        name: Token,
        methods: Vec<Stmt>,
    },

    Enum {
        name: Token,
        variants: Vec<Token>,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    None
}

#[derive(Debug, Clone, PartialEq)]
pub enum VarKind {
    Mut,
    Const,
    Fluid,
    Error
}

#[derive(Debug, Clone, PartialEq)]
pub enum VarType {
    Int,
    Float,
    Str,
    Char,
    Bool,
    Named(Token),
    Error
}