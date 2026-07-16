use crate::token::{TokenType, Token};
use crate::error::{LexerError};
use std::string::String;

pub struct Lexer {
    source: String,
    tokens: Vec<Token>,
    current: usize,
    line: usize,
    start: usize,
}

impl Lexer {

    // ! -- main functions --

    pub fn new(source: String) -> Lexer {
        Lexer {
            source,
            tokens: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
        }
    }

    pub fn scan_tokens(&mut self) -> Result<Vec<Token>, LexerError> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token()?;
        }
        self.tokens.push(Token::new(TokenType::Eof, "".to_string(), self.current, self.current)); // line | v0.4+
        Ok(self.tokens.clone())
    }

    // ! -- scan + identify --
    
    fn identifier(&mut self){
        while Self::is_alphanumeric(self.peek()) {
            self.advance();
        }

        let text = &self.source[self.start..self.current];
        let token_type = match text {

            "if" => TokenType::If,
            "elif" => TokenType::Elif,
            "else" => TokenType::Else,
            "true" => TokenType::True,
            "false" => TokenType::False,
            "for" => TokenType::For,
            "while" => TokenType::While,
            "fn" => TokenType::Func,
            "null" => TokenType::Null,
            "print" => TokenType::Print,
            "return" => TokenType::Return,
            "this" => TokenType::This,
            "let" => TokenType::Let,
            "break" => TokenType::Break,
            "continue" => TokenType::Continue,
            "import" => TokenType::Import,
            "from" => TokenType::From,
            "const" => TokenType::Const,
            "struct" => TokenType::Struct,
            "impl" => TokenType::Impl,
            "enum" => TokenType::Enum,
            "mutp" => TokenType::Mutp,
            "mut" => TokenType::Mut,
            "int" => TokenType::IntType,
            "float" => TokenType::FloatType,
            "str" => TokenType::StrType,
            "char" => TokenType::CharType,
            "bool" => TokenType::BoolType,
            "self" => TokenType::SelfKw,
            _ => TokenType::Identifier

        };
        self.add_token(token_type)
    }

    fn scan_token(&mut self) -> Result<(), LexerError> {
        let c: char = self.advance();
        match c {
            '(' => Ok(self.add_token(TokenType::LeftParen)),
            ')' => Ok(self.add_token(TokenType::RightParen)),
            '{' => Ok(self.add_token(TokenType::LeftBrace)),
            '}' => Ok(self.add_token(TokenType::RightBrace)),
            '[' => Ok(self.add_token(TokenType::LeftBracket)),
            ']' => Ok(self.add_token(TokenType::RightBracket)),
            ',' => Ok(self.add_token(TokenType::Comma)),
            '.' => Ok(self.add_token(TokenType::Dot)),
            ';' => Ok(self.add_token(TokenType::Semicolon)),

            '!' => {
                if self.match_next('=') {
                    Ok(self.add_token(TokenType::BangEqual))
                } else {
                    Ok(self.add_token(TokenType::Bang))
                }
            }

            '=' => {
                if self.match_next('=') {
                    Ok(self.add_token(TokenType::EqualEqual))
                } else {
                    Ok(self.add_token(TokenType::Equal))
                }
            }

            '>' => {
                if self.match_next('=') {
                    Ok(self.add_token(TokenType::GreaterEqual))
                } else {
                    Ok(self.add_token(TokenType::Greater))
                }
            }

            '<' => {
                if self.match_next('=') {
                    Ok(self.add_token(TokenType::LessEqual))
                } else {
                    Ok(self.add_token(TokenType::Less))
                }
            }

            '+' => {
                if self.match_next('=') {
                    Ok(self.add_token(TokenType::PlusEqual))
                } else if self.match_next('+') {
                    Ok(self.add_token(TokenType::PlusPlus))
                } else {
                    Ok(self.add_token(TokenType::Plus))
                }
            }

            '-' => {
                if self.match_next('=') {
                    Ok(self.add_token(TokenType::MinusEqual))
                } else if self.match_next('-') {
                    Ok(self.add_token(TokenType::MinusMinus))
                } else if self.match_next('>'){
                    Ok(self.add_token(TokenType::Arrow))
                } else {
                    Ok(self.add_token(TokenType::Minus))
                }
            }

            '*' => {
                if self.match_next('*') {
                    if self.match_next('=') {
                        Ok(self.add_token(TokenType::StarStarEqual))
                    } else {
                        Ok(self.add_token(TokenType::StarStar))
                    }
                } else if self.match_next('=') {
                    Ok(self.add_token(TokenType::StarEqual))
                } else {
                    Ok(self.add_token(TokenType::Star))
                }
            }
            
            '/' => {
                if self.match_next('/') {
                    Ok(while self.peek() != '\n' && !self.is_at_end() {self.advance();})
                } else if self.match_next('=') {
                    Ok(self.add_token(TokenType::SlashEqual))
                } else {
                    Ok(self.add_token(TokenType::Slash))
                }
            }

            '%' => {
                if self.match_next('=') {
                    Ok(self.add_token(TokenType::PercentEqual))
                } else {
                    Ok(self.add_token(TokenType::Percent))
                }
            }

            '&' => {
                if self.match_next('&') {
                    Ok(self.add_token(TokenType::And))
                } else {
                    Err(LexerError {
                        span: self.start..self.current, 
                        message: "Unexpected '&'.".to_string(),}
                    )
                }
            }
            '|' => {
                if self.match_next('|') {
                    Ok(self.add_token(TokenType::Or))
                } else {
                    Err(LexerError {
                        span: self.start..self.current, 
                        message: "Unexpected '|'.".to_string(),}
                    )
                }
            }
            
            ':' => if self.match_next(':') {
                    Ok(self.add_token(TokenType::ColonColon))
                } else {
                    Ok(self.add_token(TokenType::Colon))
                },

            ' ' | '\r' | '\t' => Ok({}),

            '\n' => Ok(self.line  += 1),

            '"' => {Ok(self.string()?)}

            '\'' => {self.char()},

            '0'..='9' => {Ok(self.number())},

            'a'..='z' | 'A'..='Z' | '_' => Ok(self.identifier()),

            _ => Err(LexerError {
                span: self.start..self.current, 
                message: "Expected expression.".to_string(),}
            ),

        }
    }

    // ! -- other guts and parts of the mechanism --

    fn is_at_end(&self) -> bool {
        return self.current >= self.source.len();
    }

    fn advance(&mut self) -> char {
        let c = self.source.chars().nth(self.current).unwrap();
        self.current += 1;
        return c;
    }

    fn add_token(&mut self, token_type: TokenType) {
        let lexeme: String = self.source[self.start..self.current].to_string();
        let token = Token::new(token_type, lexeme, self.start, self.current); // line | v0.4+
        self.tokens.push(token);
    }

    fn peek(&self) -> char {
        if self.is_at_end() {return '\0';}
        else {return  self.source.chars().nth(self.current).unwrap();}
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            return '\0';
        } else {
            return self.source.chars().nth(self.current + 1).unwrap();
        }
    }

    fn match_next(&mut self, expected: char) -> bool {
        if self.is_at_end() {return false;}
        if self.source.chars().nth(self.current).unwrap() != expected {return false;}

        self.current+=1;
        return true;
    }

    fn string(&mut self) -> Result<(), LexerError> {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() != '\n' {
                self.advance();
            } else {
                self.line += 1;
                self.advance();
            }
        }
        if self.peek() != '"' || self.is_at_end() {
            return Err(LexerError {
                span: self.start..self.current,
                message: "Expeced '\"' at the end of string".to_string()
        })}
        self.advance();
        
        let lexeme: String = self.source[self.start+1..self.current-1].to_string();
        let token = Token::new(TokenType::StringLit, lexeme,  self.start, self.current); // line | v0.4+
        self.tokens.push(token);
        Ok(())
    }

    fn char(&mut self) -> Result<(), LexerError> {
        self.advance();
        if self.peek() == '\'' {
            self.advance();
            Ok(self.add_token(TokenType::CharLit))
        } else {
            return Err(LexerError { 
                span: self.start..self.current, 
                message: "Char must be a single character in ''.".to_string()
            })
        }
    }

    fn is_digit(c: char) -> bool {
        return c >= '0' && c <= '9';
    }

    
    fn is_alpha(c: char) -> bool {
        if c.is_alphabetic() || c == '_' {
            return true;
        } else {return false;}
    }

    fn is_alphanumeric(c: char) -> bool {
        if Self::is_alpha(c) || Self::is_digit(c) {
            return true;
        } else {return false;}
    }

    fn number(&mut self) {
        let mut is_float = false;
        while Self::is_digit(self.peek()) {
            self.advance();
        }
        if self.peek() == '.' && Self::is_digit(self.peek_next()) {
            is_float = true;
            self.advance();
            while Self::is_digit(self.peek()) {
                self.advance();
            }
        }

        if is_float {
            self.add_token(TokenType::FloatLit);   
        } else {
            self.add_token(TokenType::IntLit);
        }
    }
}

// ! tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let mut lexer = Lexer::new("&&".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::And);

        let mut lexer = Lexer::new("||".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Or);

        let mut lexer = Lexer::new("let".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Let);

        let mut lexer = Lexer::new("break".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Break);

        let mut lexer = Lexer::new("null".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Null);

        let mut lexer = Lexer::new("mut".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Mut);
    }

    #[test]
    fn test_strings(){
        let mut lexer = Lexer::new("\"hello\"".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::StringLit);
    }

    #[test]
    fn test_operators(){
        
        let mut lexer = Lexer::new("++".to_string());
        let tokens = lexer .scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::PlusPlus);

        let mut lexer = Lexer::new("!=".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::BangEqual);
    
        let mut lexer = Lexer::new("*=".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::StarEqual);

        let mut lexer = Lexer::new("**=".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::StarStarEqual);
    }

    #[test]
    fn test_numbers(){
        let mut lexer = Lexer::new("12345".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::IntLit);
    
        let mut lexer = Lexer::new("6.0".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::FloatLit);

        let mut lexer = Lexer::new("228".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::IntLit);

        let mut lexer = Lexer::new("3.14".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::FloatLit);
    }

    #[test]
    fn test_comments(){
        let mut lexer = Lexer::new("// comment".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Eof);

        let mut lexer = Lexer::new("// _comment //".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Eof);
    }

    #[test]
    fn test_struct() {
        let mut lexer = Lexer::new("struct".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Struct);
    }

    #[test]
    fn test_colon() {
        let mut lexer = Lexer::new("let x: int;".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[2].token_type, TokenType::Colon);
    }

    #[test]
    fn test_colon_colon() {
        let mut lexer = Lexer::new("Color::Red".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].token_type, TokenType::ColonColon);
        assert_eq!(tokens[2].token_type, TokenType::Identifier);
    }

    #[test]
    fn test_mutp() {
        let mut lexer = Lexer::new("mutp".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Mutp);
    }

    #[test]
    fn test_self_tok() {
        let mut lexer = Lexer::new("self".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::SelfKw);
    }

    #[test]
    fn test_char_none_err() {
        let mut lexer = Lexer::new("let const x: char = '';".to_string());
        let tokens = lexer.scan_tokens();
        match tokens {
            Err(e) => assert!(e.message.contains("Char must be a single character in ''.")),
            Ok (_) => panic!("Expected error.")
        }
    }

    #[test]
    fn test_other(){
        let mut lexer = Lexer::new("let x = 15;\nlet y = \"y\";".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Let);
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].token_type, TokenType::Equal);
        assert_eq!(tokens[3].token_type, TokenType::IntLit);
        assert_eq!(tokens[4].token_type, TokenType::Semicolon);
        assert_eq!(tokens[5].token_type, TokenType::Let);
        assert_eq!(tokens[6].token_type, TokenType::Identifier);
        assert_eq!(tokens[7].token_type, TokenType::Equal);
        assert_eq!(tokens[8].token_type, TokenType::StringLit);
        assert_eq!(tokens[9].token_type, TokenType::Semicolon);
        assert_eq!(tokens[10].token_type, TokenType::Eof);

        let mut lexer = Lexer::new("let x = true;".to_string());
        let tokens = lexer.scan_tokens().unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Let);
        assert_eq!(tokens[0].token_type, TokenType::Let);
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].token_type, TokenType::Equal);
        assert_eq!(tokens[3].token_type, TokenType::True);
        assert_eq!(tokens[4].token_type, TokenType::Semicolon);
    }
}