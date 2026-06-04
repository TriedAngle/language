use crate::Span;

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexErrorKind {
    InvalidChar,
    UnterminatedString,
    UnterminatedBlockComment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // literals / names
    Number,
    Str,
    Ident,
    Underscore,

    // keywords
    KwLet,
    KwFn,
    KwIf,
    KwElse,
    KwMatch,
    KwStruct,
    KwReturn,
    KwFor,
    KwWhile,
    KwBreak,
    KwContinue,

    // brackets / punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semi,
    Dot,
    DotDot,
    Eq,
    FatArrow,
    Arrow,
    Caret,
    Question,
    Union,

    // operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Bang,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Amp,
    AmpAmp,
    Pipe,
    PipePipe,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    Error(LexErrorKind),
    Eof,
}
