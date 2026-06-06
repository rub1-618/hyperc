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
    
    Literal {
        value: LiteralValue,
    },
    
    Grouping {
        expr: Box<Expr>,
    },

    Variable {
        name: Token,
    }
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
        initializer: Option<Box<Expr>>,
        condition: Option<Box<Expr>>,
        increment: Option<Box<Expr>>,
        statements: Vec<Stmt>,
    },

    Return {
        value: Box<Expr>,
    },

    Function {
        name: Token,
        params: Vec<Token>,
        statements: Vec<Stmt>,
    },

    Class {
        name: Token,
        superclass: Option<Token>,
        methods: Vec<Stmt>,
    },
}

#[derive(Debug, Clone)]
pub enum LiteralValue {
    Number(f64),
    String(String),
    Bool(bool),
    Null,
}