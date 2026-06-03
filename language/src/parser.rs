use crate::Span;

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    // literals / names
    Int,
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
    Eq,
    FatArrow,
    Arrow,
    Caret,
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
    AmpAmp,
    PipePipe,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    Eof,
}
