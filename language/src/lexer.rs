use crate::{LexErrorKind, Span, Token, TokenKind};

pub struct Lexer<'src> {
    src: &'src [u8],
    pos: usize,
    last: TokenKind,
    pending: Option<Token>,
}

struct Trivia {
    crossed_newline: bool,
    semi_pos: usize,
    error: Option<Token>,
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
            pending: None,
        }
    }

    pub fn next_token(&mut self) -> Token {
        if let Some(token) = self.pending.take() {
            self.last = token.kind;
            return token;
        }

        let trivia = self.skip_trivia();
        if trivia.crossed_newline && is_terminator(self.last) && !self.next_is_closer() {
            if let Some(error) = trivia.error {
                self.pending = Some(error);
            }
            self.last = TokenKind::Semi;
            let at = trivia.semi_pos as u32;
            return Token {
                kind: TokenKind::Semi,
                span: Span { start: at, end: at },
            };
        }

        if let Some(error) = trivia.error {
            self.last = error.kind;
            return error;
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
            b'0'..=b'9' => self.number(),
            b if is_ident_start(b) => {
                self.take_while(is_ident_continue);
                keyword_or_ident(&self.src[start..self.pos])
            }
            b'"' => {
                self.advance();
                if self.string_body() {
                    TokenKind::Str
                } else {
                    TokenKind::Error(LexErrorKind::UnterminatedString)
                }
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
            b'.' => {
                self.advance();
                if self.eat_byte(b'.') {
                    TokenKind::DotDot
                } else {
                    TokenKind::Dot
                }
            }
            b'^' => self.single(TokenKind::Caret),
            b'?' => self.single(TokenKind::Question),
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
                if self.eat_byte(b'&') {
                    TokenKind::AmpAmp
                } else {
                    TokenKind::Amp
                }
            }
            b'|' => {
                self.advance();
                if self.eat_byte(b'|') {
                    TokenKind::PipePipe
                } else {
                    TokenKind::Pipe
                }
            }
            _ => {
                self.advance();
                TokenKind::Error(LexErrorKind::InvalidChar)
            }
        }
    }

    fn number(&mut self) -> TokenKind {
        self.take_number_digits();

        if self.eat_byte(b'r') {
            self.take_radix_digits();
            return TokenKind::Number;
        }

        let next = self.src.get(self.pos + 1).copied();
        if self.peek_byte() == Some(b'.') && next.is_some_and(|b| b.is_ascii_digit()) {
            self.advance();
            self.take_number_digits();
        }

        if matches!(self.peek_byte(), Some(b'e') | Some(b'E')) {
            self.advance();
            if matches!(self.peek_byte(), Some(b'+') | Some(b'-')) {
                self.advance();
            }
            self.take_number_digits();
        }

        TokenKind::Number
    }

    fn skip_trivia(&mut self) -> Trivia {
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
                    let start = self.pos;
                    if !self.block_comment(&mut newline) {
                        return Trivia {
                            crossed_newline: newline,
                            semi_pos: start,
                            error: Some(Token {
                                kind: TokenKind::Error(LexErrorKind::UnterminatedBlockComment),
                                span: Span {
                                    start: start as u32,
                                    end: self.pos as u32,
                                },
                            }),
                        };
                    }
                }
                _ => break,
            }
        }
        Trivia {
            crossed_newline: newline,
            semi_pos: self.pos,
            error: None,
        }
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

    fn take_number_digits(&mut self) {
        self.take_while(|b| b.is_ascii_digit() || b == b'_');
    }

    fn take_radix_digits(&mut self) {
        self.take_while(|b| b.is_ascii_alphanumeric() || b == b'_');
    }

    fn block_comment(&mut self, newline: &mut bool) -> bool {
        self.pos += 2;
        let mut depth = 1;
        while let Some(b) = self.peek_byte() {
            if b == b'/' && self.src.get(self.pos + 1) == Some(&b'*') {
                self.pos += 2;
                depth += 1;
                continue;
            }
            if b == b'*' && self.src.get(self.pos + 1) == Some(&b'/') {
                self.pos += 2;
                depth -= 1;
                if depth == 0 {
                    return true;
                }
                continue;
            }
            if b == b'\n' {
                *newline = true;
            }
            self.pos += 1;
        }
        false
    }

    fn string_body(&mut self) -> bool {
        while let Some(b) = self.peek_byte() {
            self.pos += 1;
            match b {
                b'\\' => {
                    if self.pos < self.src.len() {
                        self.pos += 1;
                    }
                }
                b'"' => return true,
                _ => {}
            }
        }
        false
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
        Number
            | Str
            | Ident
            | Underscore
            | RParen
            | RBracket
            | RBrace
            | KwReturn
            | KwBreak
            | KwContinue
    )
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || is_non_ascii(b)
}

fn is_ident_continue(b: u8) -> bool {
    let __ = 10;
    b.is_ascii_alphanumeric() || b == b'_' || is_non_ascii(b)
}

fn is_non_ascii(b: u8) -> bool {
    b >= 0x80
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
        b"_" => Underscore,
        b"v" => Union, // union type operator (this steals `v` as an identifier)
        _ => Ident,
    }
}
