use crate::error::{ParseError};
use crate::token::{TokenType, Token};
use crate::ast::{Expr, LiteralValue, Stmt, VarKind, VarType};

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

    pub fn parse(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut statements = vec![];
        while !self.is_at_end() {
            statements.push(self.statement()?);
        }
        return Ok(statements)
    }

    // -- recrusive descent --

    fn expression(&mut self) -> Result<Expr, ParseError> {
        return self.logicor();
    }

    // or

    fn logicor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.logicand()?;

        while self.match_token(&[TokenType::Or]) {
            let operator = self.previous().clone();
            let right = self.logicand()?;
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right), };
        }
    
        Ok(expr)    
    }

    // and

    fn logicand(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.equality()?;

        while self.match_token(&[TokenType::And]) {
            let operator = self.previous().clone();
            let right = self.equality()?;
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right), };
        }
    
        Ok(expr)    
    }

    // equality

    fn equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr: Expr = self.comparison()?;

        while self.match_token(&[TokenType::BangEqual, TokenType::EqualEqual]) {
            let operator = self.previous().clone();
            let right = self.comparison()?;
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right), };
        }

        Ok(expr)
    }

    // comparison

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr: Expr  = self.term()?;

        while self.match_token(&[TokenType::Greater, TokenType::GreaterEqual, TokenType::Less, TokenType::LessEqual]) {
            let operator = self.previous().clone();
            let right = self.term()?;
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right), };
        }

        Ok(expr)
    }

    // term

    fn term(&mut self) -> Result<Expr, ParseError> {
    let mut expr: Expr  = self.factor()?;

        while self.match_token(&[TokenType::Plus, TokenType::Minus]) {
            let operator = self.previous().clone();
            let right = self.factor()?;
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right), };
        }

        Ok(expr)
    }

    // factor

    fn factor(&mut self) -> Result<Expr, ParseError> {
    let mut expr: Expr  = self.unary()?;

        while self.match_token(&[TokenType::Slash, TokenType::Star, TokenType::Percent]) {
            let operator = self.previous().clone();
            let right = self.unary()?;
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right), };
        }

        Ok(expr)
    }

    // unary

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.match_token(&[TokenType::Bang, TokenType::Minus]) {
            let operator = self.previous().clone();
            let right: Expr = self.unary()?;
            return Ok(Expr::Unary { operator, right: Box::new(right), })
        }

       Ok(self.call()?)
    }

    // call

    fn call(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(&[TokenType::LeftParen]) {
                expr = self.finish_call(expr)?;
            } else { break; }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> Result<Expr, ParseError> {
        let mut arguments = vec![];
        if !self.check(TokenType::RightParen) {
            arguments.push(self.expression()?);
            while self.match_token(&[TokenType::Comma]) {
                arguments.push(self.expression()?);
            }
        }

        let paren: Token = self.consume(TokenType::RightParen, "Expected ')' for the function.")?;
        Ok( Expr::Call { callee: Box::new(callee), arguments, paren } )
    }

    // primary
    
    fn primary(&mut self) -> Result<Expr, ParseError> {
        if self.match_token(&[TokenType::False]) {
            return Ok(Expr::Literal { value: LiteralValue::Bool(false), span: self.previous().start..self.previous().end })
        }
        if self.match_token(&[TokenType::True]) {
            return Ok(Expr::Literal { value: LiteralValue::Bool(true), span: self.previous().start..self.previous().end })
        }
        if self.match_token(&[TokenType::Null]) {
            return Ok(Expr::Literal { value: LiteralValue::Null, span: self.previous().start..self.previous().end })
        }

        if self.match_token(&[TokenType::StringLit]) {
            return Ok(Expr::Literal { value: LiteralValue::String(self.previous().lexeme.clone()), span: self.previous().start..self.previous().end })
        }

        if self.match_token(&[TokenType::CharLit]) {
            return Ok(Expr::Literal { value: LiteralValue::Char(self.previous().lexeme.chars().nth(1).unwrap()), span: self.previous().start..self.previous().end })
        }

        if self.match_token(&[TokenType::IntLit]) {
            return Ok(Expr::Literal { value: LiteralValue::Int(self.previous().lexeme.parse::<i64>().unwrap()), span: self.previous().start..self.previous().end })
        }

        if self.match_token(&[TokenType::FloatLit]) {
            return Ok(Expr::Literal { value: LiteralValue::Float(self.previous().lexeme.parse::<f64>().unwrap()), span: self.previous().start..self.previous().end })
        }

        if self.match_token(&[TokenType::Identifier]) {
            return Ok(Expr::Variable { name: self.previous().clone() })
        }

        if self.match_token(&[TokenType::LeftParen]) {    
        let expr: Expr  = self.expression()?;
        self.consume(TokenType::RightParen, "Expect ')' after expr.")?;
        return Ok(Expr::Grouping { expr: Box::new(expr) })
        }

        Err( ParseError { 
            span: self.peek().start..self.peek().end, 
            message: "Expected expression.".to_string(),
        })
    }

    // ! -- statements matching --

    fn statement(&mut self) -> Result<Stmt, ParseError> {

        // print

        if self.match_token(&[TokenType::Print]) {
            return Ok(self.print_statement()?)
        }
        
        // declaration

        if self.match_token(&[TokenType::Let]) {
            return Ok(self.let_declaration_statement()?)
        }

        // assignment

        if self.check(TokenType::Identifier) && self.check_next(TokenType::Equal) {
            return Ok(self.assignment_statement()?)
        }

        //block

        if self.match_token(&[TokenType::LeftBrace]) {
            return Ok(self.block_statement()?)
        }

        // if

        if self.match_token(&[TokenType::If]) {
            return Ok(self.if_statement()?)
        }

        // while 

        if self.match_token(&[TokenType::While]) {
            return Ok(self.while_statement()?)
        }

        // for

        if self.match_token(&[TokenType::For]) {
            return Ok(self.for_statement()?)
        }

        // return 

        if self.match_token(&[TokenType::Return]) {
            return Ok(self.return_statement()?)
        }

        // func

        if self.match_token(&[TokenType::Func]) {
            return Ok(self.func_declaration()?)
        }

        // class

        if self.match_token(&[TokenType::Class]) {
            return Ok(self.class_declaration()?)
        }

        // else -> expr

        return Ok(self.expression_statement()?)
    }

        // ! statements fn

    // print

    fn print_statement(&mut self) -> Result<Stmt, ParseError> {
        let value: Expr = self.expression()?;
        self.consume(TokenType::Semicolon, "Expected ';' after value.")?;
        return Ok(Stmt::Print { value: Box::new(value) })
    }

    // let

    fn let_declaration_statement(&mut self) -> Result<Stmt, ParseError> {
        let kind = if self.match_token(&[TokenType::Mut]) {
            VarKind::Mut
        } else if self.match_token(&[TokenType::Const]) {
            VarKind::Const
        } else {
            return Err( ParseError { 
                span: self.peek().start..self.peek().end, 
                message: "No variable kind specified. Suggest adding 'mut' / 'const'".to_string(),
            })
        };

        let name: Token = self.consume(TokenType::Identifier, "Variable name expected.")?;
        self.consume(TokenType::Colon, "Expected ':' before type declaration.")?;

        let var_type = self.parse_type()?;
        
        let mut value: Expr = Expr::Literal { value: LiteralValue::Null, span: self.previous().start..self.previous().end  };
        if self.match_token(&[TokenType::Equal]) {
            value = self.expression()?; 
        }

        self.consume(TokenType::Semicolon, "Expected ';' after value.")?;
        return Ok(Stmt::Let {  name, value: Box::new( value ), kind, var_type });
    }

    // assignment

    fn assignment_statement(&mut self) -> Result<Stmt, ParseError> {
        let name: Token = self.consume(TokenType::Identifier, "Existing variable name expected.")?;
    
        self.consume(TokenType::Equal, "Equality sign missing.")?;
        let value: Expr = self.expression()?;
        
        self.consume(TokenType::Semicolon, "Expected ';' after value.")?;
        return Ok(Stmt::Assign {  name, value: Box::new( value ) })
    }        

    // block

    fn block_statement(&mut self) -> Result<Stmt, ParseError> {
        let mut statements: Vec<Stmt> = vec![];        
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.statement()?)
        }

        self.consume(TokenType::RightBrace, "Expected '}' after block.")?;
        return Ok(Stmt::Block { statements })
    }

    // if

    fn if_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenType::LeftParen, "Expected '(' in if statement.")?;
        let condition = self.expression()?;
        self.consume(TokenType::RightParen, "Expected ')' in if statement.")?;
        let then_branch = self.statement()?;
        let else_branch = if self.match_token(&[TokenType::Else]) {
            Some(Box::new(self.statement()?))
        } else {
            None
        };
        
        return Ok(Stmt::If { params: Box::new(condition), then_branch: Box::new(then_branch), else_branch })
    }

    // while

    fn while_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenType::LeftParen, "Expected '(' in while statement.")?;
        let conditions = self.expression()?;
        self.consume(TokenType::RightParen, "Expected ')' in while statement.")?;
        let statements = self.statement()?;
        return Ok(Stmt::While { conditions: Box::new(conditions), statements: Box::new(statements) })
    }

    // for

    fn for_statement(&mut self) -> Result<Stmt, ParseError> {
        self.consume(TokenType::LeftParen, "Expected '(' in for statement.")?;
        
        let initializer = if self.check(TokenType::Semicolon) {
            None
        } else if self.match_token(&[TokenType::Let]) {
            Some(Box::new(self.let_declaration_statement()?))
        } else {
            Some(Box::new(self.expression_statement()?))
        };

        let condition = if self.check(TokenType::Semicolon) {
            None
        } else {
            Some(Box::new(self.expression()?))
        };

        self.consume(TokenType::Semicolon, "Expected ';' after value.")?;

        let increment = if self.check(TokenType::RightParen) {
            None
        } else {
            Some(Box::new(self.expression()?))
        };

        self.consume(TokenType::RightParen, "Expected ')' in for statement.")?;

        let statements = Box::new(self.statement()?);
        return Ok(Stmt::For { initializer, condition, increment, statements })
    }

    // return

    fn return_statement(&mut self) -> Result<Stmt, ParseError> {
        let value = if self.check(TokenType::Semicolon) {
            None
        } else {
            Some(Box::new(self.expression()?))
        };
        
        self.consume(TokenType::Semicolon, "Expected ';' after return statement.")?;
        return  Ok(Stmt::Return { value });
    }

    // func

    fn func_declaration(&mut self) -> Result<Stmt, ParseError> {
        let name = self.consume(TokenType::Identifier, "No identifier for the function specified.")?;
        self.consume(TokenType::LeftParen, "Expected '(' in function statement.")?;
        let mut params: Vec<(Token, VarType)> = vec![];
        if !self.check(TokenType::RightParen) {
            let params_name= self.consume(TokenType::Identifier, "Expected type after value, consider adding, like 'a: int'")?;
            self.consume(TokenType::Colon,  "Expected ':' in function params after value.")?;
            let var_type = self.parse_type()?;
            params.push((params_name, var_type));

            while self.match_token(&[TokenType::Comma]) {
                let params_name= self.consume(TokenType::Identifier, "Expected type after value, consider adding, like 'a: int'")?;
                self.consume(TokenType::Colon,  "Expected ':' in function params after value.")?;
                let var_type = self.parse_type()?;
                params.push((params_name, var_type));
            }
        }
        self.consume(TokenType::RightParen, "Expected ')' in function statement.")?;
        
        // -> 
        let return_type = if self.match_token(&[TokenType::Arrow]) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume(TokenType::LeftBrace, "Expected '{' in function body.")?;
        let statements = Box::new(self.block_statement()?);
        return Ok(Stmt::Function { name, params, statements, return_type })
    }

    // class

    fn class_declaration(&mut self) -> Result<Stmt, ParseError> {
        let name = self.consume(TokenType::Identifier, "No identifier for the class specified.")?;
        let superclass = if self.match_token(&[TokenType::LeftParen]) {
            let sc = self.consume(TokenType::Identifier, "Expected value in function params after ','")?;
            self.consume(TokenType::RightParen, "Expected ')' after superclass identifier.")?;
            Some(sc)
        } else {
            None
        };

        self.consume(TokenType::LeftBrace, "Expected '{' before class body.")?;

        let mut methods = vec![];
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            self.consume(TokenType::Func, "Expected 'func' before the method identifier.")?;
            methods.push(self.func_declaration()?);
        }

        self.consume(TokenType::RightBrace, "Expected '}' after class body.")?;
        return Ok(Stmt::Class { name, superclass, methods })
    }

    // else: expression

    fn expression_statement(&mut self) -> Result<Stmt, ParseError> {
        let expr: Expr = self.expression()?;
        self.consume(TokenType::Semicolon, "Expected ';' after value.")?;
        return Ok(Stmt::Expression { value: Box::new(expr) });
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

    fn parse_type(&mut self) -> Result<VarType, ParseError> {
        if self.match_token(&[TokenType::IntType]) {
            return Ok(VarType::Int)
        } else if self.match_token(&[TokenType::FloatType]) {
            return Ok(VarType::Float)
        } else if self.match_token(&[TokenType::StrType]) {
            return Ok(VarType::Str)
        } else if self.match_token(&[TokenType::CharType]) {
            return Ok(VarType::Char)
        } else if self.match_token(&[TokenType::BoolType]) {
            return Ok(VarType::Bool)
            
            // todo
        // } if self.match_token(&[TokenType::ArrType]) {
 
        } else {
            return Err( ParseError { 
                span: self.peek().start..self.peek().end, 
                message: "Expected variable type.".to_string(),
            })
        };
    }
    

    fn consume(&mut self, token_type:TokenType, message: &str) -> Result<Token, ParseError> {
        if self.check(token_type) {
            Ok(self.advance().clone())
        } else {
            Err(ParseError { span: self.peek().start..self.peek().end, message: message.to_string() })
        }
    }
    
}

// ! -- tests 1 --

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

// ! -- tests 2 --

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    #[test]
    fn test_decl() {
        let mut lexer = Lexer::new("let const x: int = 5;".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());
        let stmt = _parser.parse().unwrap();
        assert!(matches!(stmt[0], Stmt::Let { .. }));
    }

    #[test]
    fn test_print() {
        let mut lexer = Lexer::new("print(x);".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());
        let stmt = _parser.parse().unwrap();
        assert!(matches!(stmt[0], Stmt::Print { .. }));
    }
    
    #[test]
    fn test_assign() {
        let mut lexer = Lexer::new("x = 7;".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());
        let stmt = _parser.parse().unwrap();
        assert!(matches!(stmt[0], Stmt::Assign { .. }));
    }
    
    #[test]
    fn test_block() {
        let mut lexer = Lexer::new("{ let mut x: str = \"hello from block!\"; }".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());
        let stmt = _parser.parse().unwrap();
        assert!(matches!(stmt[0], Stmt::Block { .. }));
    }
    
    #[test]
    fn test_while() {
        let mut lexer = Lexer::new("while (x < 10) { x + 1; }".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());
        let stmt = _parser.parse().unwrap();
        assert!(matches!(stmt[0], Stmt::While { .. }));
    }

    #[test]
    fn test_for() {
        let mut lexer = Lexer::new("for (let mut i: int = 0; i < 20; i + 1) { print(\"hello\"); }".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());
        let stmt = _parser.parse().unwrap();
        assert!(matches!(stmt[0], Stmt::For { .. }));
    }
    
    #[test]
    fn test_func() {
        let mut lexer = Lexer::new("func foo( a: int ) { print( a ); }".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());
        let stmt = _parser.parse().unwrap();
        assert!(matches!(stmt[0], Stmt::Function { .. }));
    }

    #[test]
    fn test_return() {
        let mut lexer = Lexer::new("return a + b;".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());
        let stmt = _parser.parse().unwrap();
        assert!(matches!(stmt[0], Stmt::Return { .. }));
    }

    #[test]
    fn test_class() {
        let mut lexer = Lexer::new("class Dog (Animal) { func bark() { print(\"bark\"); } }".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());
        let stmt = _parser.parse().unwrap();
        assert!(matches!(stmt[0], Stmt::Class { .. }));
    }

    #[test]
    fn test_stacked() {
        let mut lexer = Lexer::new("class Dog (Animal) { func bark() { let mut a: str = \"meow\"; a = \"bark\"; print( a ); } }".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        let mut _parser = Parser::new(tokens.clone());
        let stmt = _parser.parse().unwrap();
        assert!(matches!(stmt[0], Stmt::Class { .. }));
    }

}