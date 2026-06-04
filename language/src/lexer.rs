use crate::{Span, Token, TokenKind};

pub struct Lexer<'src> {
    src: &'src [u8],
    pos: usize,
    last: TokenKind,
}

pub struct Cursor<'src> {
    lexer: Lexer<'src>,
    current: Token,
}

impl<'src> Cursor<'src> {
    pub fn new(src: &'src str) -> Self {
        let mut lexer = Lexer::new(src);
        let current = lexer.next_token();
        Cursor { lexer, current }
    }

    pub fn peek(&self) -> Token {
        self.current
    }

    pub fn bump(&mut self) -> Token {
        let prev = self.current;
        self.current = self.lexer.next_token();
        prev
    }
}

impl<'src> Lexer<'src> {
    pub fn new(src: &'src str) -> Self {
        Lexer {
            src: src.as_bytes(),
            pos: 0,
            last: TokenKind::Semi,
        }
    }

    pub fn next_token(&mut self) -> Token {
        let crossed_newline = self.skip_trivia();
        if crossed_newline && is_terminator(self.last) && !self.next_is_closer() {
            self.last = TokenKind::Semi;
            let at = self.pos as u32;
            return Token {
                kind: TokenKind::Semi,
                span: Span { start: at, end: at },
            };
        }

        let start = self.pos;
        let kind = self.scan();
        self.last = kind;
        Token {
            kind,
            span: Span {
                start: start as u32,
                end: self.pos as u32,
            },
        }
    }

    fn scan(&mut self) -> TokenKind {
        let start = self.pos;
        let b = match self.peek_byte() {
            Some(b) => b,
            None => return TokenKind::Eof,
        };
        match b {
            b'0'..=b'9' => {
                self.take_while(|c| c.is_ascii_digit());
                TokenKind::Int
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                self.take_while(|c| c.is_ascii_alphanumeric() || c == b'_');
                keyword_or_ident(&self.src[start..self.pos])
            }
            b'"' => {
                self.advance();
                self.string_body();
                TokenKind::Str
            }
            b'(' => self.single(TokenKind::LParen),
            b')' => self.single(TokenKind::RParen),
            b'{' => self.single(TokenKind::LBrace),
            b'}' => self.single(TokenKind::RBrace),
            b'[' => self.single(TokenKind::LBracket),
            b']' => self.single(TokenKind::RBracket),
            b',' => self.single(TokenKind::Comma),
            b':' => self.single(TokenKind::Colon),
            b';' => self.single(TokenKind::Semi),
            b'.' => self.single(TokenKind::Dot),
            b'^' => self.single(TokenKind::Caret),
            b'%' => self.single(TokenKind::Percent),
            b'+' => {
                self.advance();
                if self.eat_byte(b'=') {
                    TokenKind::PlusEq
                } else {
                    TokenKind::Plus
                }
            }
            b'-' => {
                self.advance();
                if self.eat_byte(b'>') {
                    TokenKind::Arrow
                } else if self.eat_byte(b'=') {
                    TokenKind::MinusEq
                } else {
                    TokenKind::Minus
                }
            }
            b'*' => {
                self.advance();
                if self.eat_byte(b'=') {
                    TokenKind::StarEq
                } else {
                    TokenKind::Star
                }
            }
            b'/' => {
                self.advance();
                if self.eat_byte(b'=') {
                    TokenKind::SlashEq
                } else {
                    TokenKind::Slash
                }
            }
            b'=' => {
                self.advance();
                if self.eat_byte(b'>') {
                    TokenKind::FatArrow
                } else if self.eat_byte(b'=') {
                    TokenKind::EqEq
                } else {
                    TokenKind::Eq
                }
            }
            b'!' => {
                self.advance();
                if self.eat_byte(b'=') {
                    TokenKind::Ne
                } else {
                    TokenKind::Bang
                }
            }
            b'<' => {
                self.advance();
                if self.eat_byte(b'=') {
                    TokenKind::Le
                } else {
                    TokenKind::Lt
                }
            }
            b'>' => {
                self.advance();
                if self.eat_byte(b'=') {
                    TokenKind::Ge
                } else {
                    TokenKind::Gt
                }
            }
            b'&' => {
                self.advance();
                self.eat_byte(b'&');
                TokenKind::AmpAmp
            }
            b'|' => {
                self.advance();
                self.eat_byte(b'|');
                TokenKind::PipePipe
            }
            _ => {
                self.advance();
                TokenKind::Eof
            }
        }
    }

    fn skip_trivia(&mut self) -> bool {
        let mut newline = false;
        loop {
            match self.peek_byte() {
                Some(b' ') | Some(b'\t') | Some(b'\r') => self.pos += 1,
                Some(b'\n') => {
                    newline = true;
                    self.pos += 1;
                }
                Some(b'/') if self.src.get(self.pos + 1) == Some(&b'/') => {
                    while let Some(c) = self.peek_byte() {
                        if c == b'\n' {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                Some(b'/') if self.src.get(self.pos + 1) == Some(&b'*') => {
                    self.pos += 2;
                    while self.pos < self.src.len() {
                        if self.peek_byte() == Some(b'*')
                            && self.src.get(self.pos + 1) == Some(&b'/')
                        {
                            self.pos += 2;
                            break;
                        }
                        if self.peek_byte() == Some(b'\n') {
                            newline = true;
                        }
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
        newline
    }

    fn peek_byte(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn eat_byte(&mut self, b: u8) -> bool {
        if self.peek_byte() == Some(b) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn take_while(&mut self, pred: impl Fn(u8) -> bool) {
        while let Some(b) = self.peek_byte() {
            if pred(b) {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn string_body(&mut self) {
        while let Some(b) = self.peek_byte() {
            self.pos += 1;
            match b {
                b'\\' => self.pos += 1,
                b'"' => break,
                _ => {}
            }
        }
    }

    fn single(&mut self, k: TokenKind) -> TokenKind {
        self.advance();
        k
    }

    fn next_is_closer(&self) -> bool {
        matches!(self.peek_byte(), Some(b')') | Some(b']') | Some(b'}'))
    }
}

fn is_terminator(k: TokenKind) -> bool {
    use TokenKind::*;
    matches!(
        k,
        Int | Str | Ident | RParen | RBracket | RBrace | KwReturn | KwBreak | KwContinue
    )
}

fn keyword_or_ident(s: &[u8]) -> TokenKind {
    use TokenKind::*;
    match s {
        b"let" => KwLet,
        b"fn" => KwFn,
        b"if" => KwIf,
        b"else" => KwElse,
        b"match" => KwMatch,
        b"struct" => KwStruct,
        b"return" => KwReturn,
        b"while" => KwWhile,
        b"for" => KwFor,
        b"break" => KwBreak,
        b"continue" => KwContinue,
        b"v" => Union, // union type operator (this steals `v` as an identifier)
        _ => Ident,
    }
}
