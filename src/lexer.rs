use crate::token::{Token, TokenType};
use crate::error::{HypercError};
use std::string::String;

pub struct Lexer {
    source: String,
    tokens: Vec<Token>,
    current: usize,
    start: usize,
    line: usize,
    pub errors: Vec<HypercError>,
}

impl Lexer {
    
    // ! -- main funcs --

    pub fn new(source: String) -> Self {
        Lexer { source, tokens: Vec::new(), current: 0, start: 0, line: 1, errors: Vec::new() }
    }

    pub fn scan_tokens(&mut self) -> Vec<Token> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token();
        }
        self.tokens.push(Token { token_type: TokenType::Eof, lexeme: "".to_string(), start: self.current, end: self.current });
        self.tokens.clone()
    }

    // ! -- matchings --

    fn scan_token(&mut self) {
        let c: char = self.advance();
        match c {
            '(' => self.add_token(TokenType::LeftParen),
            ')' => self.add_token(TokenType::RightParen),
            '{' => self.add_token(TokenType::LeftBrace),
            '}' => self.add_token(TokenType::RightBrace),
            '[' => self.add_token(TokenType::LeftBracket),
            ']' => self.add_token(TokenType::RightBracket),
            ',' => self.add_token(TokenType::Comma),
            '.' => self.add_token(TokenType::Dot),
            ';' => self.add_token(TokenType::Semicolon),

            '+' => self.add_token(TokenType::Plus),
            '*' => self.add_token(TokenType::Star),
            '%' => self.add_token(TokenType::Percent),

            '!' => {
                if self.match_next('=') {
                    self.add_token(TokenType::BangEqual);
                } else {
                    self.add_token(TokenType::Bang);
                }
            }

            '=' => {
                if self.match_next('=') {
                    self.add_token(TokenType::EqualEqual);
                } else {
                    self.add_token(TokenType::Equal);
                }
            }

            '>' => {
                if self.match_next('=') {
                    self.add_token(TokenType::GreaterEqual);
                } else {
                    self.add_token(TokenType::Greater);
                }
            }

            '<' => {
                if self.match_next('=') {
                    self.add_token(TokenType::LessEqual);
                } else {
                    self.add_token(TokenType::Less);
                }
            }

            '/' => {
                if self.match_next('/') {
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(TokenType::Slash)
                }
            },

            '-' => {
                if self.match_next('>') {
                    self.add_token(TokenType::Arrow);
                } else {
                    self.add_token(TokenType::Minus);
                }
            }


            '&' => {
                if self.match_next('&') {
                    self.add_token(TokenType::And);
                } else {
                    self.errors.push(HypercError::LexerError { 
                        span: self.start..self.current, 
                        message: "Unexpected '&'.".to_string()
                    });
                }
            }

            '|' => {
                if self.match_next('|') {
                    self.add_token(TokenType::Or);
                } else {
                    self.errors.push(HypercError::LexerError { 
                        span: self.start..self.current, 
                        message: "Unexpected '|'.".to_string()
                    });
                }
            }

            ':' => {
                if self.match_next(':') {
                    self.add_token(TokenType::ColonColon);
                } else {
                    self.add_token(TokenType::Colon);
                }
            }

            // todo: ++ | -- | ** | += | -= | *= | **=

            ' ' | '\r' | '\t' => {},

            '\n' => self.line += 1,

            '"' => self.string(),

            '\'' => self.char(),

            '0'..='9' => self.number(),

            'a'..='z' | 'A'..='Z' | '_' => self.identifier(),

            _ => self.errors.push(HypercError::LexerError { 
                span: self.start..self.current, 
                message: "Expected expression.".to_string() 
            }),
        }

    }

    fn identifier(&mut self) {
        while Self::is_identifier(self.peek()) {
            self.advance();
        }
        let text = &self.source[self.start..self.current];
        let token_type = match text {
            
            "if"        => TokenType::If,
            "else"      => TokenType::Else,

            "true"      => TokenType::True,
            "false"     => TokenType::False,

            "print"     => TokenType::Print,

            "let"       => TokenType::Let,

            "func"      => TokenType::Func,
            "return"    => TokenType::Return,
            "self"      => TokenType::SelfTok,

            "for"       => TokenType::For,
            "while"     => TokenType::While,
            "break"     => TokenType::Break,
            "continue"  => TokenType::Continue,

            "const"     => TokenType::Const,
            "mut"       => TokenType::Mut,
            "fluid"     => TokenType::Fluid,

            "struct"    => TokenType::Struct,
            "impl"      => TokenType::Impl,
            "enum"      => TokenType::Enum,

            "int"       => TokenType::IntType,
            "float"     => TokenType::FloatType,
            "str"       => TokenType::StrType,
            "char"      => TokenType::CharType,
            "bool"      => TokenType::BoolType,
            
            // "import" => TokenType::Import,
            // "from" => TokenType::From,

            _ => TokenType::Identifier,
        
        };
        self.add_token(token_type)
    }

    // ! -- helpers --
 
    fn add_token(&mut self, token_type: TokenType) {
        let lexeme = self.source[self.start..self.current].to_string();
        let token = Token::new(token_type, lexeme, self.start, self.current);
        self.tokens.push(token);
    }

    fn advance(&mut self) -> char {
        let c = self.source.chars().nth(self.current).unwrap();
        self.current += 1;
        c
    }

    fn match_next(&mut self, expected: char) -> bool {
        if self.is_at_end() { return false }
        if self.source.chars().nth(self.current).unwrap() != expected { return false }
        self.current += 1;
        true
    }

    fn peek(&self) -> char {
        if self.is_at_end() { return '\0' }
        self.source.chars().nth(self.current).unwrap()
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() { return '\0' }
        self.source.chars().nth(self.current + 1).unwrap()
    }

    fn is_at_end(&self) -> bool {
        return self.current >= self.source.len();
    }

    fn is_identifier(c: char) -> bool {
        return c.is_ascii_digit() || c.is_alphabetic() || c == '_';
    }

    // ! -- literals --

    fn string(&mut self) {
        while self.peek() != '"' && !&self.is_at_end() {
            if self.peek() != '\n' {
                self.advance();
            } else {
                self.line += 1;
                self.advance();
            }
        }
        if self.peek() != '"' || self.is_at_end() {
            self.errors.push(HypercError::LexerError { 
                span: self.start..self.current, 
                message: "Expected '\"' at the end of the string.".to_string()
            });
            return 
        }
        self.advance();

        let lexeme = self.source[self.start+1..self.current-1].to_string();
        let token = Token::new(TokenType::StringLit, lexeme, self.start, self.current);
        self.tokens.push(token);
    }

    fn char(&mut self) {
        if self.peek_next() != '\'' {
            while self.peek() != '\'' && self.peek() != '\0' {
                self.advance();
            }
            if self.peek() != '\0' {
                self.advance();
            }
            self.errors.push(HypercError::LexerError { 
                span: self.start..self.current, 
                message: "Char must be a single character in ''.".to_string()
            });
            return 
        }
        self.advance();
        self.advance();
        self.add_token(TokenType::CharLit)
    }

    fn number(&mut self) {
        while self.peek().is_ascii_digit() {
            self.advance();
        }
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance();
            while self.peek().is_ascii_digit() {
                self.advance();
            }
            self.add_token(TokenType::FloatLit)
        } else { self.add_token(TokenType::IntLit) }
    }
}

// ! TESTS

#[cfg(test)]
mod tests {
    use super::*;

    fn lex_source_ok(src: &str) -> Vec<TokenType> {
        let mut lexer = Lexer::new(src.to_string());
        let tokens = lexer.scan_tokens();
        let mut token_type_vec: Vec<TokenType> = vec![];
        for token in tokens {
            token_type_vec.push(token.token_type);
        }
        token_type_vec
    }

    fn lex_source_err(src: &str) -> Vec<HypercError> {
        let mut lexer = Lexer::new(src.to_string());
        let _tokens = lexer.scan_tokens();
        lexer.errors
    }



    #[test]
    fn test_literals_ok() {

        let tok = lex_source_ok("\"stringlit\"");
        assert_eq!(tok[0], TokenType::StringLit);

        let tok = lex_source_ok("'c'");
        assert_eq!(tok[0], TokenType::CharLit);

        let tok = lex_source_ok("5");
        assert_eq!(tok[0], TokenType::IntLit);

        let tok = lex_source_ok("3.14");
        assert_eq!(tok[0], TokenType::FloatLit);

    }

    #[test]
    fn test_operators_ok() {

        let tok = lex_source_ok(",");
        assert_eq!(tok[0], TokenType::Comma); 

        let tok = lex_source_ok(".");
        assert_eq!(tok[0], TokenType::Dot);

        let tok = lex_source_ok(";");
        assert_eq!(tok[0], TokenType::Semicolon);

        let tok = lex_source_ok("!");
        assert_eq!(tok[0], TokenType::Bang);

        let tok = lex_source_ok("!=");
        assert_eq!(tok[0], TokenType::BangEqual);

        let tok = lex_source_ok("=");
        assert_eq!(tok[0], TokenType::Equal);
        
        let tok = lex_source_ok("==");
        assert_eq!(tok[0], TokenType::EqualEqual);

        let tok = lex_source_ok(">");
        assert_eq!(tok[0], TokenType::Greater);

        let tok = lex_source_ok(">=");
        assert_eq!(tok[0], TokenType::GreaterEqual);

        let tok = lex_source_ok("<");
        assert_eq!(tok[0], TokenType::Less);

        let tok = lex_source_ok("<=");
        assert_eq!(tok[0], TokenType::LessEqual);

        let tok = lex_source_ok("+");
        assert_eq!(tok[0], TokenType::Plus);

        let tok = lex_source_ok("-");
        assert_eq!(tok[0], TokenType::Minus);

        let tok = lex_source_ok("*");
        assert_eq!(tok[0], TokenType::Star);  

        let tok = lex_source_ok("/");
        assert_eq!(tok[0], TokenType::Slash);

        let tok = lex_source_ok("%");
        assert_eq!(tok[0], TokenType::Percent);

        let tok = lex_source_ok("->");
        assert_eq!(tok[0], TokenType::Arrow);

        let tok = lex_source_ok(":");
        assert_eq!(tok[0], TokenType::Colon);

        let tok = lex_source_ok("::");
        assert_eq!(tok[0], TokenType::ColonColon);

        let tok = lex_source_ok("&&");
        assert_eq!(tok[0], TokenType::And);

        let tok = lex_source_ok("||");
        assert_eq!(tok[0], TokenType::Or);

    }

    #[test]
    fn test_id_and_kw_ok() {

        let tok = lex_source_ok("x");
        assert_eq!(tok[0], TokenType::Identifier);

        let tok = lex_source_ok("if");
        assert_eq!(tok[0], TokenType::If);

        let tok = lex_source_ok("else");
        assert_eq!(tok[0], TokenType::Else);

        let tok = lex_source_ok("true");
        assert_eq!(tok[0], TokenType::True);

        let tok = lex_source_ok("false");
        assert_eq!(tok[0], TokenType::False);

        let tok = lex_source_ok("self");
        assert_eq!(tok[0], TokenType::SelfTok);

        let tok = lex_source_ok("for");
        assert_eq!(tok[0], TokenType::For);

        let tok = lex_source_ok("while");
        assert_eq!(tok[0], TokenType::While);

        let tok = lex_source_ok("func");
        assert_eq!(tok[0], TokenType::Func);

        let tok = lex_source_ok("print");
        assert_eq!(tok[0], TokenType::Print);

        let tok = lex_source_ok("return");
        assert_eq!(tok[0], TokenType::Return);

        let tok = lex_source_ok("let");
        assert_eq!(tok[0], TokenType::Let);

        let tok = lex_source_ok("break");
        assert_eq!(tok[0], TokenType::Break);

        let tok = lex_source_ok("continue");
        assert_eq!(tok[0], TokenType::Continue);

        let tok = lex_source_ok("struct");
        assert_eq!(tok[0], TokenType::Struct);

        let tok = lex_source_ok("impl");
        assert_eq!(tok[0], TokenType::Impl);

        let tok = lex_source_ok("enum");
        assert_eq!(tok[0], TokenType::Enum);

        let tok = lex_source_ok("const");
        assert_eq!(tok[0], TokenType::Const);

        let tok = lex_source_ok("mut");
        assert_eq!(tok[0], TokenType::Mut);

        let tok = lex_source_ok("fluid");
        assert_eq!(tok[0], TokenType::Fluid);

        let tok = lex_source_ok("int");
        assert_eq!(tok[0], TokenType::IntType);

        let tok = lex_source_ok("float");
        assert_eq!(tok[0], TokenType::FloatType);

        let tok = lex_source_ok("str");
        assert_eq!(tok[0], TokenType::StrType);

        let tok = lex_source_ok("char");
        assert_eq!(tok[0], TokenType::CharType);

        let tok = lex_source_ok("bool");
        assert_eq!(tok[0], TokenType::BoolType);

    }

    #[test]
    fn test_eof_and_comments_ok() {
        
        let tok = lex_source_ok("");
        assert_eq!(tok[0], TokenType::Eof);

        let tok = lex_source_ok("// comment");
        assert_eq!(tok[0], TokenType::Eof);

        let tok = lex_source_ok("// _comment //");
        assert_eq!(tok[0], TokenType::Eof);

    }

    #[test]
    fn test_enclosures_ok() {

        let tok = lex_source_ok("(");
        assert_eq!(tok[0], TokenType::LeftParen);

        let tok = lex_source_ok(")");
        assert_eq!(tok[0], TokenType::RightParen);

        let tok = lex_source_ok("{");
        assert_eq!(tok[0], TokenType::LeftBrace);

        let tok = lex_source_ok("}");
        assert_eq!(tok[0], TokenType::RightBrace);

        let tok = lex_source_ok("[");
        assert_eq!(tok[0], TokenType::LeftBracket);

        let tok = lex_source_ok("]");
        assert_eq!(tok[0], TokenType::RightBracket);

    }

    #[test]
    fn test_operators_err() {

        let err = lex_source_err("&");
        match &err[0] {
            HypercError::LexerError {message, .. } => {
                assert_eq!(message, "Unexpected '&'.")
            }
            _ => panic!("Expected LexerError.")
        }

        let err = lex_source_err("|");
        match &err[0] {
            HypercError::LexerError {message, .. } => {
                assert_eq!(message, "Unexpected '|'.")
            }
            _ => panic!("Expected LexerError.")
        }

    }

    #[test]
    fn test_char_err() {

        let err = lex_source_err("'err'");
        match &err[0] {
            HypercError::LexerError {message, .. } => {
                assert_eq!(message, "Char must be a single character in ''.")
            }
            _ => panic!("Expected LexerError.")
        }

        let err = lex_source_err("''");
        match &err[0] {
            HypercError::LexerError {message, .. } => {
                assert_eq!(message, "Char must be a single character in ''.")
            }
            _ => panic!("Expected LexerError.")
        }

    }

}