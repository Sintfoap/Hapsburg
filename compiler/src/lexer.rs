use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    // literals
    Ident(String),
    IntLit(i64),
    StrLit(String),

    // keywords
    Dynasty,
    Descends,
    Founder,
    Trait,
    Override,
    Abstract,
    Birth,
    Succession,
    Over,
    As,
    Claim,
    Contested,
    Heir,
    Return,
    SelfKw,
    True,
    False,
    And,
    Or,

    // punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Colon,
    ColonColon,
    Comma,
    Dot,
    Arrow,
    Eq,
    EqEq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Pipe,
    Underscore,

    Eof,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct SpannedTok {
    pub tok: Tok,
    pub line: usize,
}

pub struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer { src: src.as_bytes(), pos: 0, line: 1 }
    }

    fn peek(&self) -> u8 {
        if self.pos < self.src.len() { self.src[self.pos] } else { 0 }
    }

    fn peek_at(&self, off: usize) -> u8 {
        let p = self.pos + off;
        if p < self.src.len() { self.src[p] } else { 0 }
    }

    fn bump(&mut self) -> u8 {
        let c = self.peek();
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
        }
        c
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek() {
                b' ' | b'\t' | b'\r' | b'\n' => {
                    self.bump();
                }
                b'/' if self.peek_at(1) == b'/' => {
                    while self.peek() != b'\n' && self.peek() != 0 {
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<SpannedTok>, String> {
        let mut out = Vec::new();
        loop {
            self.skip_trivia();
            let line = self.line;
            if self.pos >= self.src.len() {
                out.push(SpannedTok { tok: Tok::Eof, line });
                break;
            }
            let c = self.peek();
            let tok = if c.is_ascii_alphabetic() || c == b'_' {
                self.lex_ident()
            } else if c.is_ascii_digit() {
                self.lex_number()?
            } else if c == b'"' {
                self.lex_string()?
            } else {
                self.lex_punct()?
            };
            out.push(SpannedTok { tok, line });
        }
        Ok(out)
    }

    fn lex_ident(&mut self) -> Tok {
        let start = self.pos;
        while self.peek().is_ascii_alphanumeric() || self.peek() == b'_' {
            self.bump();
        }
        let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        match s {
            "dynasty" => Tok::Dynasty,
            "descends" => Tok::Descends,
            "founder" => Tok::Founder,
            "trait" => Tok::Trait,
            "override" => Tok::Override,
            "abstract" => Tok::Abstract,
            "birth" => Tok::Birth,
            "succession" => Tok::Succession,
            "over" => Tok::Over,
            "as" => Tok::As,
            "claim" => Tok::Claim,
            "contested" => Tok::Contested,
            "heir" => Tok::Heir,
            "return" => Tok::Return,
            "self" => Tok::SelfKw,
            "true" => Tok::True,
            "false" => Tok::False,
            "and" => Tok::And,
            "or" => Tok::Or,
            "_" => Tok::Underscore,
            _ => Tok::Ident(s.to_string()),
        }
    }

    fn lex_number(&mut self) -> Result<Tok, String> {
        let start = self.pos;
        while self.peek().is_ascii_digit() {
            self.bump();
        }
        let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
        s.parse::<i64>()
            .map(Tok::IntLit)
            .map_err(|e| format!("line {}: bad integer literal '{}': {}", self.line, s, e))
    }

    fn lex_string(&mut self) -> Result<Tok, String> {
        self.bump(); // opening quote
        let mut s = String::new();
        loop {
            let c = self.peek();
            if c == 0 {
                return Err(format!("line {}: unterminated string literal", self.line));
            }
            if c == b'"' {
                self.bump();
                break;
            }
            if c == b'\\' {
                self.bump();
                let esc = self.bump();
                match esc {
                    b'n' => s.push('\n'),
                    b't' => s.push('\t'),
                    b'"' => s.push('"'),
                    b'\\' => s.push('\\'),
                    other => s.push(other as char),
                }
                continue;
            }
            s.push(self.bump() as char);
        }
        Ok(Tok::StrLit(s))
    }

    fn lex_punct(&mut self) -> Result<Tok, String> {
        let c = self.bump();
        let tok = match c {
            b'{' => Tok::LBrace,
            b'}' => Tok::RBrace,
            b'(' => Tok::LParen,
            b')' => Tok::RParen,
            b'[' => Tok::LBracket,
            b']' => Tok::RBracket,
            b',' => Tok::Comma,
            b'.' => Tok::Dot,
            b'|' => Tok::Pipe,
            b'+' => Tok::Plus,
            b'-' => {
                if self.peek() == b'>' {
                    self.bump();
                    Tok::Arrow
                } else {
                    Tok::Minus
                }
            }
            b'*' => Tok::Star,
            b'/' => Tok::Slash,
            b'%' => Tok::Percent,
            b':' => {
                if self.peek() == b':' {
                    self.bump();
                    Tok::ColonColon
                } else {
                    Tok::Colon
                }
            }
            b'=' => {
                if self.peek() == b'=' {
                    self.bump();
                    Tok::EqEq
                } else {
                    Tok::Eq
                }
            }
            b'!' => {
                if self.peek() == b'=' {
                    self.bump();
                    Tok::NotEq
                } else {
                    return Err(format!("line {}: unexpected '!' (use 'not' is unsupported; only '!=' is valid)", self.line));
                }
            }
            b'<' => {
                if self.peek() == b'=' {
                    self.bump();
                    Tok::LtEq
                } else {
                    Tok::Lt
                }
            }
            b'>' => {
                if self.peek() == b'=' {
                    self.bump();
                    Tok::GtEq
                } else {
                    Tok::Gt
                }
            }
            other => {
                return Err(format!(
                    "line {}: unexpected character '{}'",
                    self.line, other as char
                ))
            }
        };
        Ok(tok)
    }
}
