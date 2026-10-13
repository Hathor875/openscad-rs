use logos::{Lexer, Logos};
use text_size::TextSize;

#[derive(Logos, Debug, Clone, PartialEq)]
pub enum Token {
    #[token("module")]
    Module,
    #[token("function")]
    Function,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("let")]
    Let,
    #[token("assert")]
    Assert,
    #[token("echo")]
    Echo,
    #[token("for")]
    For,
    #[token("each")]
    Each,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("undef")]
    Undef,
    #[token("<=")]
    Le,
    #[token(">=")]
    Ge,
    #[token("==")]
    Eq,
    #[token("!=")]
    Neq,
    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("<<")]
    Lsh,
    #[token(">>")]
    Rsh,
    #[token("include")]
    Include,
    #[token("use")]
    Use,
    // IncludePath handled in parser via string literal after include/use
    #[regex(r#""([^"\\]|\\.)*""#)]
    String,
    #[regex(r"0x[0-9a-fA-F]+")]
    HexNumber,
    #[regex(r"[0-9]*\.[0-9]+([eE][+-]?[0-9]+)?", priority = 5)]
    #[regex(r"[0-9]+\.[0-9]*([eE][+-]?[0-9]+)?", priority = 4)]
    #[regex(r"[0-9]+[eE][+-]?[0-9]+", priority = 3)]
    FloatNumber,
    #[regex(r"[0-9]+", priority = 2)]
    IntNumber,
    #[regex(r"[a-zA-Z_$][a-zA-Z0-9_$]*")]
    Ident,
    #[token(";")]
    Semi,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token("=")]
    EqSign,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("^")]
    Caret,
    #[token("<")]
    Lt,
    #[token(">")]
    Gt,
    #[token("!")]
    Bang,
    #[token("~")]
    Tilde,
    #[token("&")]
    Amp,
    #[token("|")]
    Pipe,
    #[token("?")]
    Question,
    #[token(":")]
    Colon,
    #[token(".")]
    Dot,
    #[token("#")]
    Hash,
    #[token("\x03")]
    Eot,
    #[token("\u{00A0}")]
    Nbsp,
    #[token("\u{FEFF}")]
    Bom,
    #[regex(r"[ \t\r\f]+", logos::skip)]
    #[regex(r"//[^\n]*", logos::skip)]
    #[regex(r"/\*([^*]|\*+[^*/])*\*+/", logos::skip)]
    #[token("\n")]
    Newline,
    #[error]
    Error,
}

impl Token {
    pub fn is_trivia(&self) -> bool {
        matches!(self, Token::Newline | Token::Nbsp | Token::Bom)
    }
}

pub struct LexerWithSpans<'a> {
    lexer: Lexer<'a, Token>,
    pub offset: TextSize,
}

impl<'a> LexerWithSpans<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            lexer: Token::lexer(input),
            offset: TextSize::default(),
        }
    }
}

impl<'a> Iterator for LexerWithSpans<'a> {
    type Item = (Token, TextSize, TextSize);

    fn next(&mut self) -> Option<Self::Item> {
        let token = self.lexer.next()?;
        let start = self.offset;
        let len = TextSize::try_from(self.lexer.slice().len()).ok()?;
        self.offset += len;
        Some((token, start, self.offset))
    }
}

pub fn lex(input: &str) -> Vec<(Token, TextSize, TextSize)> {
    LexerWithSpans::new(input).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let input = "module foo() { cube(10); }";
        let tokens = lex(input);
        assert!(tokens.iter().any(|(t, _, _)| matches!(t, Token::Module)));
        assert!(tokens.iter().any(|(t, _, _)| matches!(t, Token::Ident)));
        assert!(tokens.iter().any(|(t, _, _)| matches!(t, Token::IntNumber)));
    }

    #[test]
    fn test_string_literal() {
        let input = r#""hello world""#;
        let tokens = lex(input);
        assert!(matches!(tokens.first(), Some((Token::String, _, _))));
    }

    #[test]
    fn test_numbers() {
        let input = "10 3.14 1e5 0xFF";
        let tokens = lex(input);
        assert!(matches!(tokens[0].0, Token::IntNumber));
        assert!(matches!(tokens[1].0, Token::FloatNumber));
        assert!(matches!(tokens[2].0, Token::FloatNumber));
        assert!(matches!(tokens[3].0, Token::HexNumber));
    }

#[test]
    fn test_operators() {
        let input = "<= >= == != && || << >>";
        let tokens = lex(input);
        for (i, (tok, start, end)) in tokens.iter().enumerate() {
            let slice = &input[usize::from(*start)..usize::from(*end)];
            println!("Token {}: {:?} (slice: {:?})", i, tok, slice);
        }
        assert!(matches!(tokens[0].0, Token::Le));
        assert!(matches!(tokens[1].0, Token::Ge));
        assert!(matches!(tokens[2].0, Token::Eq));
        assert!(matches!(tokens[3].0, Token::Neq));
        assert!(matches!(tokens[4].0, Token::And));
        assert!(matches!(tokens[5].0, Token::Or));
        assert!(matches!(tokens[6].0, Token::Lsh));
        assert!(matches!(tokens[7].0, Token::Rsh));
    }
}