use std::string::String;
use crate::token::{Token};

#[derive(Debug, Clone)]
pub enum Expr {
    Binary {
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
    },
    
    Grouping {
        expr: Box<Expr>,
    },
    
    Variable {
        name: Token,
    },
}

#[derive(Debug, Clone)]
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
        kind: VarKind,
        var_type: VarType,
    },

    Assign {
        name: Token,
        value: Box<Expr>,
    },

    Block {
        statements: Vec<Stmt>,
    },

    If {
        params: Box<Expr>,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },

    While {
        conditions: Box<Expr>,
        statements: Box<Stmt>,
    },

    For {
        initializer: Option<Box<Stmt>>,
        condition: Option<Box<Expr>>,
        increment: Option<Box<Expr>>,
        statements: Box<Stmt>,
    },

    Return {
        value: Option<Box<Expr>>,
    },

    Function {
        name: Token,
        params: Vec<(Token, VarType)>,
        statements: Box<Stmt>,
        return_type: Option<VarType>,
    },

    Class {
        name: Token,
        superclass: Option<Token>,
        methods: Vec<Stmt>,
    },
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone)]
pub enum VarKind {
    Mut,
    Const,
}

#[derive(Debug, Clone)]
pub enum VarType {
    Int,
    Float,
    Str,
    Char,
    Bool,
}