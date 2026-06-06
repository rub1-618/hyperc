use crate::token::{TokenType, Token};
use crate::ast::{Stmt, Expr, LiteralValue};

#[derive(Debug, Clone)]
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {

    // ! -- main functions --

    pub fn new(tokens: Vec<Token>) -> Parser {
        Parser {
            tokens,
            current: 0,
        }
    }

    pub fn parse(&mut self) -> Vec<Stmt> {
        let mut statements = vec![];
        while !self.is_at_end() {
            statements.push(self.statement());
        }
        return statements;
    }

    // -- recrusive descent --

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

    // ! -- statements matching --

    fn statement(&mut self) -> Stmt {
        
        // print

        if self.match_token(&[TokenType::Print]) {
            return self.print_statement()
        }
        
        // declaration

        if self.match_token(&[TokenType::Let]) {
            return self.let_declaration_statement()
        }

        // assignment

        if self.check(TokenType::Identifier) && self.check_next(TokenType::Equal) {
            return self.assignment_statement();
        }

        //block

        if self.match_token(&[TokenType::LeftBrace]) {
            return self.block_statement();
        }

        // if

        if self.match_token(&[TokenType::If]) {
            return self.if_statement();
        }

        // while 

        if self.match_token(&[TokenType::While]) {
            return self.while_statement();
        }

        // else -> expr

        return self.expression_statement();
    }

        // ! statements fn

    // print

    fn print_statement(&mut self) -> Stmt {
        let value: Expr = self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' after value.");
        return Stmt::Print { value: Box::new(value) };
    }

    // let

    fn let_declaration_statement(&mut self) -> Stmt {
        let name: Token = self.consume(TokenType::Identifier, "Variable name expected.").clone();


        let mut value: Expr = Expr::Literal { value: LiteralValue::Null };
        if self.match_token(&[TokenType::Equal]) {
            value = self.expression(); 
        }

        self.consume(TokenType::Semicolon, "Expected ';' after value.");
        return Stmt::Let {  name, value: Box::new( value ) };
    }

    // assignment

    fn assignment_statement(&mut self) -> Stmt {
        let name: Token = self.consume(TokenType::Identifier, "Existing variable name expected.").clone();
    
        self.consume(TokenType::Equal, "Equality sign missing.");
        let value: Expr = self.expression();
        
        self.consume(TokenType::Semicolon, "Expected ';' after value.");
        return Stmt::Assign {  name, value: Box::new( value ) };
    }        

    // block

    fn block_statement(&mut self) -> Stmt {
        let mut statements: Vec<Stmt> = vec![];        
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.statement())
        }

        self.consume(TokenType::RightBrace, "Expected '}' after block.");
        return Stmt::Block { statements };
    }

    // if

    fn if_statement(&mut self) -> Stmt {
        self.consume(TokenType::LeftParen, "Expected '(' in if statement.");
        let condition = self.expression();
        self.consume(TokenType::RightParen, "Expected ')' in if statement.");
        let then_branch = self.statement();
        let else_branch = if self.match_token(&[TokenType::Else]) {
            Some(Box::new(self.statement()))
        } else {
            None
        };
        
        return Stmt::If { params: Box::new(condition), then_branch: Box::new(then_branch), else_branch }
    }

    // while

    fn while_statement(&mut self) -> Stmt {
        self.consume(TokenType::LeftParen, "Expected '(' in while statement.");
        let conditions = self.expression();
        self.consume(TokenType::RightParen, "Expected ')' in while statement.");
        let statements = self.statement();
        return Stmt::While { conditions: Box::new(conditions), statements: Box::new(statements) };
    }

    fn return_statement(&mut self) -> Stmt {
        value = Box::new(self.expression());
        self.consume(TokenType::Semicolon, "Expected ';' after return statement.");
    }

    // else: expression

    fn expression_statement(&mut self) -> Stmt {
        let expr: Expr = self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' after value.");
        return Stmt::Expression { value: Box::new(expr) };
    }

    // ! -- the guts and other details of the parser --

    fn peek(&self) -> &Token {
        return &self.tokens[self.current];
    }

    fn peek_next(&self) -> &Token {
        return &self.tokens[self.current + 1];
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

    fn check_next(&self, token_type: TokenType) -> bool {
        if self.is_at_end() {return false}
        return self.peek_next().token_type == token_type;
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

    fn consume(&mut self, token_type:TokenType, message: &str) -> &Token {
        if self.check(token_type) {
            return self.advance();
        }

        panic!("{}", message);
    }
    
}

// -- tests --

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::lexer::Lexer;

//     #[test]
//     fn test_binary_addition() {
//         let mut lexer = Lexer::new("1 + 2".to_string());
//         let tokens = lexer.scan_tokens();
//         let mut parser = Parser::new(tokens);
//         let expr = parser.parse();
//         assert!(matches!(expr, Expr::Binary { .. }));
//     }

//     #[test]
//     fn test_unary() {
//         let mut lexer = Lexer::new("-5".to_string());
//         let tokens = lexer.scan_tokens();
//         let mut parser = Parser::new(tokens);
//         let expr = parser.parse();
//         assert!(matches!(expr, Expr::Unary { .. }));
//     }

//     #[test]
//     fn test_grouping() {
//         let mut lexer = Lexer::new("(1 + 2)".to_string());
//         let tokens = lexer.scan_tokens();
//         let mut parser = Parser::new(tokens);
//         let expr = parser.parse();
//         assert!(matches!(expr, Expr::Grouping { .. }));
//     }

//     #[test]
//     fn test_literal_number() {
//         let mut lexer = Lexer::new("7".to_string());
//         let tokens = lexer.scan_tokens();
//         let mut parser = Parser::new(tokens);
//         let expr = parser.parse();
//         assert!(matches!(expr, Expr::Literal { .. }));
//     }

//     #[test]
//     fn test_comparison() {
//         let mut lexer = Lexer::new("1 > 2".to_string());
//         let tokens = lexer.scan_tokens();
//         let mut parser = Parser::new(tokens);
//         let expr = parser.parse();
//         assert!(matches!(expr, Expr::Binary { .. }));
//     }

//     #[test]
//     fn test_literal_bool() {
//         let mut lexer = Lexer::new("true".to_string());
//         let tokens = lexer.scan_tokens();
//         let mut parser = Parser::new(tokens);
//         let expr = parser.parse();
//         assert!(matches!(expr, Expr::Literal { .. }));
//     }

//     #[test]
//      fn test_literal_str() {
//         let mut lexer = Lexer::new("\"hello\"".to_string());
//         let tokens = lexer.scan_tokens();
//         let mut parser = Parser::new(tokens);
//         let expr = parser.parse();
//         assert!(matches!(expr, Expr::Literal { .. }));
//      }   

//      #[test]
//      fn test_precedence() {
//         let mut lexer = Lexer::new("1 + 2 * 3".to_string());
//         let tokens = lexer.scan_tokens();
//         let mut parser = Parser::new(tokens);
//         let expr = parser.parse();
//         if let Expr::Binary { right, .. } = expr {
//             assert!(matches!(*right, Expr::Binary { .. }))
//         }
//      }  

// }