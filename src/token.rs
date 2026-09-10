#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    
    // Single character tokens
    LeftParen, RightParen, LeftBrace, RightBrace,                   // | () | {} | : |
    LeftBracket, RightBracket, Comma, Dot, Semicolon,               // | [] | , | . | ; |

    // 1 or 2 char tokens
    Bang, BangEqual,                                                // | ! | != |
    Equal, EqualEqual,                                              // | = | == |
    Greater, GreaterEqual,                                          // | > | >= |
    Less, LessEqual,                                                // | < | <= |
    Plus,                                                           // | + |
    Minus,                                                          // | - |
    Star,                                                           // | * |
    Slash,                                                          // | / |
    Percent,                                                        // | % | 
    Arrow,                                                          // | -> |
    Colon, ColonColon,                                              // | : | :: |       

    // Literals
    Identifier, StringLit, CharLit, IntLit, FloatLit, 

    // Keywords
    And, Or, If, Else, True, False, SelfTok,                        // | && | || | if(){} | else{} | true | false | self |
    For, While, Func,  Print, Return, Let,                          // | for(){} | while(){} | func(){} | print() | return ... | let |
    Break, Continue, Struct, Enum, Impl,                            // | break | continue | const ... | struct{} | enum | impl |

    // Kinds
    Const,
    Mut,
    Fluid,

    // Types
    IntType,                                                        // | int |
    FloatType,                                                      // | float |
    StrType,                                                        // | str |
    CharType,                                                       // | char |
    BoolType,                                                       // | bool |
    // todo ArrType,                                                        // | arr |

    // PlusPlus, PlusEqual, MinusMinus, 
    // MinusEqual, StarStar, StarEqual, 
    // StarStarEqual, SlashEqual, PercentEqual,
    // Import, From, 

    Error,

    Eof
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn new(token_type: TokenType, lexeme: String, start: usize, end: usize) -> Self {
        Token { token_type, lexeme, start, end }
    }
}