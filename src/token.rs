#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {

    // Single character tokens
    LeftParen, RightParen, LeftBrace, RightBrace,            // | () | {} | : |
    LeftBracket, RightBracket, Comma, Dot, Semicolon,               // | [] | , | . | ; |

    // 1 or 2 char tokens
    Bang, BangEqual,                                                // | ! | != | (works as NOT too)
    Equal, EqualEqual,                                              // | = | == |
    Greater, GreaterEqual,                                          // | > | >= |
    Less, LessEqual,                                                // | < | <= |
    Plus, PlusPlus, PlusEqual,                                      // | + | ++ | += |
    Minus, MinusMinus, MinusEqual,                                  // | - | -- | -= |
    Star, StarStar, StarEqual, StarStarEqual,                       // | * | ** | *= | **= |
    Slash, SlashEqual,                                              // | / | /= |
    Percent, PercentEqual,                                          // | % | %= |
    Arrow,                                                          // | -> |
    Colon, ColonColon,                                              // | : | :: |       

    // Literals
    Identifier, StringLit, CharLit, IntLit, FloatLit, 

    // Keywords
    And, Or, Class, If, Elif, Else, True, False,                    // | && | || | class(){} | if(){} | elif(){} | else{} | true | false |
    For, While, Func, Null, Print, Return, This, Let,               // | for(){} | while(){} | func(){} | null | print() | return ... | this | let |
    Break, Continue, Import, From, Struct, Enum, Impl,              // | break | continue | import ... | import ... from ... | const ... | struct{} | enum | impl |

    // Kinds
    Const,
    Mut,
    Mutp,

    // Types
    IntType,                                                        // | int |
    FloatType,                                                      // | float |
    StrType,                                                        // | str |
    CharType,                                                       // | char |
    BoolType,                                                       // | bool |
    // todo ArrType,                                                        // | arr |

    // Hyper-Rust only
    // ...

    Eof
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

impl Token {
    pub fn new(token_type: TokenType, lexeme: String, line: usize, start: usize, end: usize ) -> Self {
        Token { token_type, lexeme, line, start, end  }
    }
}