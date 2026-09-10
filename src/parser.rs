use crate::token::{Token, TokenType};
use crate::error::{HypercError};
use crate::ast::{
    Expr, Stmt,
    LiteralValue,
    VarKind, VarType
};

#[derive(Debug, Clone)]
struct Parser {
    tokens: Vec<Token>,
    current: usize,
    errors: Vec<HypercError>
}

impl Parser {

    // ! -- main functions --

    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            current: 0,
            errors: Vec::new()
        }
    }

    pub fn parse(&mut self) -> Vec<Stmt> {
        let mut stmts = vec![];
        while !self.is_at_end() {
            stmts.push(self.statement());
        }
        stmts
    }

    // ! -- statements matching --

    fn statement(&mut self) -> Stmt {

        // print

        if self.match_token(&[TokenType::Print]) {
            return self.print_statement()
        }

        // let (declaration)

        else if self.match_token(&[TokenType::Let]) {
            return self.if_statement()
        }

        // block { }

        else if self.match_token(&[TokenType::LeftBrace]) {
            return self.block_statement()
        }

        // if => else

        else if self.match_token(&[TokenType::If]) {
           return self.if_statement()
        }

        // while

        else if self.match_token(&[TokenType::While]) {
           return self.while_statement()
        }

        // for

        else if self.match_token(&[TokenType::For]) {
           return self.for_statement()
        }

        // func

        else if self.match_token(&[TokenType::Func]) {
           return self.func_statement()
        }

        // return

        else if self.match_token(&[TokenType::Return]) {
           return self.return_statement()
        }

        // struct

        else if self.match_token(&[TokenType::Struct]) {
           return self.struct_statement()
        }

        // impl

        else if self.match_token(&[TokenType::Impl]) {
           return self.impl_statement()
        }

        // enum

        else if self.match_token(&[TokenType::Enum]) {
           return self.enum_statement()
        }

        // else => expr
        else {
            return self.expression_statement()
        }
    }

    // print

    fn print_statement(&mut self) -> Stmt {
        let value = self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' after print().");
        Stmt::Print { value: Box::new(value) }
    }

    // let (decl)

    fn let_declaration_statement(&mut self) -> Stmt {
        let var_kind = if self.match_token(&[TokenType::Const]) {
            VarKind::Const
        } else if self.match_token(&[TokenType::Mut]) {
            VarKind::Mut
        } else if self.match_token(&[TokenType::Fluid]) {
            VarKind::Fluid
        } else {
            self.errors.push(HypercError::ParseError { 
                span: self.peek().start..self.peek().end,
                message: "No variable kind specified. Suggest adding 'mut' / 'const' / 'fluid'.".to_string()
            });
            VarKind::Error
        };

        let name = self.consume(TokenType::Identifier, "Expected variable name.");
        self.consume(TokenType::Colon, "Expected ':' before type declaration.");

        let var_type = self.parse_type();

        let mut value: Expr = Expr::Literal { 
            value: LiteralValue::None, 
            span: self.peek_prev().start..self.peek_prev().end 
        };
        if self.match_token(&[TokenType::Equal]) {
            value = self.expression();
        }

        self.consume(TokenType::Semicolon, &"Expected ';' after value.");
        Stmt::Let { name, value: Box::new(value), var_kind, var_type }
    }

    // block { }

    fn block_statement(&mut self) -> Stmt {
        let mut statements = vec![];
        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.statement());
        }
        self.consume(TokenType::RightBrace, "Expected '}' at the end of the block.");
        Stmt::Block { statements }
    }

    // if => else

    fn if_statement(&mut self) -> Stmt {
        self.consume(TokenType::LeftParen, "Expected '(' for if condition.");
        let condition = self.expression();
        self.consume(TokenType::RightParen, "Expected ')' for if condition.");
        let then_branch = self.statement();
        let else_branch = if self.match_token(&[TokenType::Else]) {
            Some(Box::new(self.statement()))
        } else {
            None
        };
        Stmt::If { 
            condition: Box::new(condition), 
            then_branch: Box::new(then_branch), 
            else_branch
        }
    }

    // while

    fn while_statement(&mut self) -> Stmt {
        self.consume(TokenType::LeftParen, "Expected '(' for while condition.");
        let condition = self.expression();
        self.consume(TokenType::RightParen, "Expected ')' for while condition.");
        let statements = self.statement();
        Stmt::While { condition: Box::new(condition), statements: Box::new(statements) }
    }

    // for

    fn for_statement(&mut self) -> Stmt {
        self.consume(TokenType::LeftParen, "Expected '(' in for statement.");
        
        let initializer = if self.check(TokenType::Semicolon) {
            None
        } else if self.match_token(&[TokenType::Let]) {
            Some(Box::new(self.let_declaration_statement()))
        } else {
            Some(Box::new(self.expression_statement()))
        };

        let condition = if self.check(TokenType::Semicolon) {
            None
        } else {
            let expr = Some(Box::new(self.expression()));
            self.consume(TokenType::Semicolon, "Expected ';' after value.");
            expr
        };

        let increment = if self.check(TokenType::RightParen) {
            None
        } else {
            Some(Box::new(self.statement()))
        };

        self.consume(TokenType::RightParen, "Expected ')' in for statement.");

        let statements = self.statement();
        Stmt::For { initializer, condition, increment, statements: Box::new(statements) }
    }

    // func

    fn func_statement(&mut self) -> Stmt {
        let name = self.consume(TokenType::Identifier, "Expected identifier for a function.");
        self.consume(TokenType::LeftParen, "Expected '(' before function arguments.");
        let mut args = vec![];
        if !self.check(TokenType::RightParen) {
            let arg_name = self.consume(TokenType::Identifier, "Expected an argument name.");
            self.consume(TokenType::Colon, "Expected ':' after an argument.");
            let var_type = self.parse_type();
            args.push((arg_name, var_type));        
        
            while self.match_token(&[TokenType::Comma]) {
                let arg_name = self.consume(TokenType::Identifier, "Expected an argument name.");
                self.consume(TokenType::Colon, "Expected ':' after an argument.");
                let var_type = self.parse_type();
                args.push((arg_name, var_type));
            }
        }
        self.consume(TokenType::RightParen, "Expected ')' after function arguments.");

        // ->
        let return_type = if self.match_token(&[TokenType::Arrow]) {
            Some(self.parse_type())
        } else {
            None
        };
    
        self.consume(TokenType::LeftBrace, "Expected '{' in function body.");
        let statements = Box::new(self.block_statement());
        Stmt::Func { name, args, statements, return_type }

    }

    // return

    fn return_statement(&mut self) -> Stmt {
        let value = if self.check(TokenType::Semicolon) {
            None
        } else {
            Some(Box::new(self.expression()))
        };

        self.consume(TokenType::Semicolon, "Expected ';' after return.");
        Stmt::Return { value }
    }

    // struct

    fn struct_statement(&mut self) -> Stmt {
        let name = self.consume(TokenType::Identifier, "Expected struct name.");
        self.consume(TokenType::LeftBrace, "Expected '{' in struct statement.");
        let mut fields = vec![];
        while !self.check(TokenType::RightBrace) {
            let field_name = self.consume(TokenType::Identifier, "Expected field in struct statement.");
            self.consume(TokenType::Colon, "Expected ':' after struct field.");
            let var_type = self.parse_type();
            fields.push((field_name, var_type));
            if !self.match_token(&[TokenType::Comma]) {
                break;
            }
        }
        self.consume(TokenType::RightBrace, "Expected '}' in struct statement.");
        Stmt::Struct { name, fields }
    
    }

    // impl

    fn impl_statement(&mut self) -> Stmt {
        let name = self.consume(TokenType::Identifier, "Expected impl name");
        self.consume(TokenType::LeftBrace, "Expected '{' in impl statement.");
        let mut methods = vec![];
        while !self.check(TokenType::RightBrace) {
            self.consume(TokenType::Func, "Expected method in impl.");
            methods.push(self.func_statement());
        }
        self.consume(TokenType::RightBrace, "Expected '}' in impl statement.");
        Stmt::Impl { name, methods }
    }

    // enum

    fn enum_statement(&mut self) -> Stmt {
        let name = self.consume(TokenType::Identifier, "Expected enum name");
        self.consume(TokenType::LeftBrace, "Expected '{' in enum statement.");
        let mut variants = vec![];
        while !self.check(TokenType::RightBrace) {
            let var_name = self.consume(TokenType::Identifier, "Expected variant in enum.");
            variants.push(var_name);
            if !self.match_token(&[TokenType::Comma]) {
                break;
            }
        }
        self.consume(TokenType::RightBrace, "Expected '}' in enum statement.");
        Stmt::Enum { name, variants }
    }

    // expr

    fn expression_statement(&mut self) -> Stmt {
        let expr = self.expression();
        if self.match_token(&[TokenType::Equal]) {
            let value = self.expression();
            match expr {
                Expr::Variable {..} | 
                Expr::Get {..} => {
                    self.consume(TokenType::Semicolon, "Expected ';' after assignment.");
                    return Stmt::Assign { target: Box::new(expr), value: Box::new(value) };
                }
                _ => self.errors.push(HypercError::ParseError { 
                    span: self.peek_prev().start..self.peek_prev().end, 
                    message: "Invalid assignment target.".to_string()
                })
            }
        }
        self.consume(TokenType::Semicolon, "Expected ';' after expression.");
        Stmt::Expression { value: Box::new(expr) }
    }


    // ! -- expr recursive descent --

    fn expression(&mut self) -> Expr {
        self.binary_logic_or()
    }

    fn binary_logic_or(&mut self) -> Expr {
        let mut expr = self.binary_logic_and();

        while self.match_token(&[
                TokenType::Or
            ]) {
            let operator = self.peek_prev().clone();
            let right = self.binary_logic_and();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right) }
        }
        expr
    }

    fn binary_logic_and(&mut self) -> Expr {
        let mut expr = self.binary_equality();

        while self.match_token(&[
                TokenType::And
            ]) {
            let operator = self.peek_prev().clone();
            let right = self.binary_equality();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right) }
        }
        expr
    }

    fn binary_equality(&mut self) -> Expr {
        let mut expr = self.binary_comparison();

        while self.match_token(&[
                TokenType::EqualEqual, TokenType::BangEqual
            ]) {
            let operator = self.peek_prev().clone();
            let right = self.binary_comparison();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right) }
        }
        expr
    }

    fn binary_comparison(&mut self) -> Expr {
        let mut expr = self.binary_term();

        while self.match_token(&[
                TokenType::Greater, TokenType::GreaterEqual,
                TokenType::Less, TokenType::LessEqual
            ]) {
            let operator = self.peek_prev().clone();
            let right = self.binary_term();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right) }
        }
        expr
    }

    fn binary_term(&mut self) -> Expr {
        let mut expr = self.binary_factor();

        while self.match_token(&[
                TokenType::Plus, TokenType::Minus
            ]) {
            let operator = self.peek_prev().clone();
            let right = self.binary_factor();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right) }
        }
        expr
    }

    fn binary_factor(&mut self) -> Expr {
        let mut expr = self.unary();

        while self.match_token(&[
                TokenType::Star, TokenType::Slash
            ]) {
            let operator = self.peek_prev().clone();
            let right = self.unary();
            expr = Expr::Binary { left: Box::new(expr), operator, right: Box::new(right) }
        }
        expr
    }

    fn unary(&mut self) -> Expr {
        while self.match_token(&[
                TokenType::Bang, TokenType::Minus
            ]) {
            let operator = self.peek_prev().clone();
            let right = self.unary();
            return Expr::Unary { operator, right: Box::new(right) }
        }
        self.call()
    }

    fn call(&mut self) -> Expr {
        let mut expr = self.primary();

        loop {
            if self.match_token(&[TokenType::LeftParen]) {
                expr = self.finish_call(expr);
            } else if self.match_token(&[TokenType::Dot]) {
                expr = self.finish_get(expr);
            } else { break; }
        }
        expr
    }

        fn finish_get(&mut self, expr: Expr) -> Expr {
            let field = self.consume(TokenType::Identifier, "Expected an identifier after '.'.");
            Expr::Get { object: Box::new(expr), field }
        }

        fn finish_call(&mut self, callee: Expr) -> Expr {
            let mut arguments = vec![];
            if !self.check(TokenType::RightParen) {
                arguments.push(self.expression());
                while self.match_token(&[TokenType::Comma]) {
                    arguments.push(self.expression());
                }
            }
            let paren = self.consume(TokenType::RightParen, "Expected ')' in function call.");
            Expr::Call { callee: Box::new(callee), arguments, paren }
        }

    fn primary(&mut self) -> Expr {
        if self.match_token(&[TokenType::IntLit]) {
            Expr::Literal { 
                value: LiteralValue::Int(self.parse_int()), 
                span: self.peek_prev().start..self.peek_prev().end
            }
        }
        else if self.match_token(&[TokenType::FloatLit]) {
            Expr::Literal { 
                value: LiteralValue::Float(self.parse_float()), 
                span: self.peek_prev().start..self.peek_prev().end 
            }
        }
        else if self.match_token(&[TokenType::StringLit]) {
            Expr::Literal { 
                value: LiteralValue::String(self.peek_prev().lexeme.clone()), 
                span: self.peek_prev().start..self.peek_prev().end 
            }
        }
        else if self.match_token(&[TokenType::CharLit]) {
            Expr::Literal { 
                value: LiteralValue::Char(self.peek_prev().lexeme.chars().nth(1).unwrap()), 
                span: self.peek_prev().start..self.peek_prev().end 
            }
        }
        else if self.match_token(&[TokenType::True]) {
            Expr::Literal { 
                value: LiteralValue::Bool(true), 
                span: self.peek_prev().start..self.peek_prev().end 
            }
        } 
        else if self.match_token(&[TokenType::False]) {
            Expr::Literal { 
                value: LiteralValue::Bool(false), 
                span: self.peek_prev().start..self.peek_prev().end 
            }
        }

        else if self.match_token(&[TokenType::SelfTok]) {
            Expr::SelfExpr { self_tok: self.peek_prev().clone() }
        }

        else if self.match_token(&[TokenType::Identifier]) {
            
            // structlit

            if self.check(TokenType::LeftBrace) {
                let name = self.peek_prev().clone();
                let mut fields: Vec<(Token, Box<Expr>)> = vec![];
                self.consume(TokenType::LeftBrace, "Expected '{' in struct literal.");
                while !self.check(TokenType::RightBrace) {
                    let field_name = self.consume(TokenType::Identifier, "Expected field name.");
                    self.consume(TokenType::Colon, "Expected ':' in struct literal after field name.");
                    let expr = self.expression();
                    fields.push((field_name, Box::new(expr)));
                    if self.match_token(&[TokenType::Comma]) {
                        break
                    }
                }
                self.consume(TokenType::RightBrace, "Expected '}' in struct literal.");
                Expr::StructLit { name, fields }
            } else if self.check(TokenType::ColonColon) {
                let type_name = self.peek_prev().clone();
                self.consume(TokenType::ColonColon, "Expected '::' in path.");
                let item = self.consume(TokenType::Identifier, "Expected identifier in path after '::'.");
                Expr::Path { type_name, item }
            } else {
                Expr::Variable { name: self.peek_prev().clone() }
            }

        } else {
            self.errors.push(HypercError::ParseError { 
                span: self.peek().start..self.peek().end,
                message: "Expected expression".to_string()
            });
            Expr::ErrorExpr
        }
    }

    // ! -- helpers --

    fn match_token(&mut self, token_types: &[TokenType]) -> bool {
        for token_type in token_types {
            if self.check(token_type.clone()) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        return self.peek_prev();
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn peek_next(&self) -> &Token {
        &self.tokens[self.current + 1]
    }

    fn peek_prev(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn check(&self, token_type: TokenType) -> bool {
        if self.is_at_end() { return false }
        return self.peek().token_type == token_type
    }

    fn check_next(&self, token_type: TokenType) -> bool {
        if self.is_at_end() { return false }
        return self.peek_next().token_type == token_type
    }

    fn is_at_end(&self) -> bool {
        self.peek().token_type == TokenType::Eof
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> Token {
        if self.check(token_type) {
            self.advance().clone()
        } else {
            self.errors.push(HypercError::ParseError { 
                span: self.peek().start..self.peek().end, 
                message: message.to_string()
            });
            // self.advance();
            let err_tok = Token { 
                token_type: TokenType::Error, 
                lexeme: "".to_string(), 
                start: self.peek().start, 
                end: self.peek().end
            };
            return err_tok;
        }
    }

    fn parse_type(&mut self) -> VarType {
        if self.match_token(&[TokenType::IntType]) {
            VarType::Int
        } else if self.match_token(&[TokenType::FloatType]) {
            VarType::Float
        } else if self.match_token(&[TokenType::StrType]) {
            VarType::Str
        } else if self.match_token(&[TokenType::CharType]) {
            VarType::Char
        } else if self.match_token(&[TokenType::BoolType]) {
            VarType::Bool
        } else if self.match_token(&[TokenType::Identifier]) {
            VarType::Named(self.peek_prev().clone())
        } else {
            self.errors.push(HypercError::ParseError { 
                span: self.peek_prev().start..self.peek_prev().end,
                message: "Expected variable type.".to_string()
            });
            VarType::Error
        }
    }

    fn parse_int(&mut self) -> i64 {
        match self.peek_prev().lexeme.parse::<i64>() {
            Ok(i) => i,
            Err(_) => {
                self.errors.push(HypercError::ParseError { 
                    span: self.peek_prev().start..self.peek_prev().end,
                    message: "Integer literal is too large.".to_string()
                });
                0
            }
        }       
    }

    fn parse_float(&mut self) -> f64 {
        match self.peek_prev().lexeme.parse::<f64>() {
            Ok(f) => f,
            Err(_) => {
                self.errors.push(HypercError::ParseError { 
                    span: self.peek_prev().start..self.peek_prev().end,
                    message: "Float literal is too large.".to_string()
                });
                0.0
            }
        }       
    }
}