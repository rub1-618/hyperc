use std::string::String;
use crate::token::{TokenType, Token};
use crate::ast::{Expr, LiteralValue};

#[derive(Debug, Clone)]
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser {
            tokens,
            current: 0,
        }
    }

    fn peek(&self) -> &Token {
        return &self.tokens[self.current];
    }

    fn previous(&self) -> &Token {
        return &self.tokens[self.current - 1];
    }

    fn is_at_end(&self) -> bool {
        if self.peek().token_type == TokenType::Eof {
            return true;
        } else {
            return false;
        }
    }

    fn advance(&mut self) -> &Token {
       if !self.is_at_end() {
            self.current += 1;
        }
        return self.previous();
    }

    fn check(&self, token_type: TokenType) -> bool {
        if self.is_at_end() {return false}
        return self.peek().token_type == token_type;
    }

    fn match_token(&mut self, token_types: &[TokenType]) -> bool {
        for token_type in token_types {
            if self.check(token_type.clone()) {
                self.advance();
                return true;
            }
        }

        return false;
    }

    // decrusive descent

    fn expression(&mut self) -> Expr {
        return self.equality();
    }

    // equality

    fn equality(&mut self) -> Expr {
        let mut expr: Expr  = self.comparison();

        while self.match_token(&[TokenType::BangEqual, TokenType::EqualEqual]) {
            let operator = self.previous().clone();
            let right = self.comparison();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right), };
        }

        return expr;
    }

    // comparison

    fn comparison(&mut self) -> Expr {
        let mut expr: Expr  = self.term();

        while self.match_token(&[TokenType::Greater, TokenType::GreaterEqual, TokenType::Less, TokenType::LessEqual]) {
            let operator = self.previous().clone();
            let right = self.term();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right), };
        }

        return expr;
    }

    // term

    fn term(&mut self) -> Expr {
    let mut expr: Expr  = self.factor();

        while self.match_token(&[TokenType::Plus, TokenType::Minus]) {
            let operator = self.previous().clone();
            let right = self.factor();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right), };
        }

        return expr;
    }

    // factor

    fn factor(&mut self) -> Expr {
    let mut expr: Expr  = self.unary();

        while self.match_token(&[TokenType::Slash, TokenType::Star, TokenType::Percent]) {
            let operator = self.previous().clone();
            let right = self.unary();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right), };
        }

        return expr;
    }

    // unary

    fn unary(&mut self) -> Expr {
        if self.match_token(&[TokenType::Bang, TokenType::Minus]) {
            let operator = self.previous().clone();
            let right: Expr = self.unary();
            return Expr::Unary { operator, right: Box::new(right), };
        }

        return self.primary();
    }

    // primary
    fn primary(&mut self) -> Expr {
        if self.match_token(&[TokenType::False]) {
            return Expr::Literal { value: LiteralValue::Bool(false)};
        }
        if self.match_token(&[TokenType::True]) {
            return Expr::Literal { value: LiteralValue::Bool(true)};
        }
        if self.match_token(&[TokenType::Null]) {
            return Expr::Literal { value: LiteralValue::Null};
        }

        if self.match_token(&[TokenType::StringLit]) {
            return Expr::Literal { value: LiteralValue::String(self.previous().lexeme.clone()) }
        }
        if self.match_token(&[TokenType::Number]) {
            return Expr::Literal { value: LiteralValue::Number(self.previous().lexeme.parse::<f64>().unwrap()) }
        }

        if self.match_token(&[TokenType::LeftParen]) {    
        let expr: Expr  = self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expr.");
        return Expr::Grouping { expr: Box::new(expr) };
        }

        panic!("Expect expr.");
    }

    fn consume(&mut self, token_type:TokenType, message: &str) -> &Token {
        if self.check(token_type) {
            return self.advance();
        }

        panic!("{}", message);
    }

    pub fn parse(&mut self) -> Expr {
        return self.expression();
    }
}
