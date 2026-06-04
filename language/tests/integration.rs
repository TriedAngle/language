use language::lexer::Lexer;
use language::{LexErrorKind, TokenKind};

fn lex_kinds(src: &str) -> Vec<TokenKind> {
    let mut lexer = Lexer::new(src);
    let mut kinds = Vec::new();
    loop {
        let token = lexer.next_token();
        kinds.push(token.kind);
        if token.kind == TokenKind::Eof {
            break;
        }
    }
    kinds
}

fn lex_token_data(src: &str) -> Vec<(TokenKind, u32, u32)> {
    let mut lexer = Lexer::new(src);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        tokens.push((token.kind, token.span.start, token.span.end));
        if token.kind == TokenKind::Eof {
            break;
        }
    }
    tokens
}

#[test]
fn lexes_utf8_symbols_as_identifiers_with_byte_spans() {
    let src = "λ😀 + café";

    assert_eq!(
        lex_token_data(src),
        vec![
            (TokenKind::Ident, 0, 6),
            (TokenKind::Plus, 7, 8),
            (TokenKind::Ident, 9, 14),
            (TokenKind::Eof, 14, 14),
        ]
    );
}

#[test]
fn lexes_single_and_double_amp_pipe_and_question() {
    assert_eq!(
        lex_kinds("& && | || ?"),
        vec![
            TokenKind::Amp,
            TokenKind::AmpAmp,
            TokenKind::Pipe,
            TokenKind::PipePipe,
            TokenKind::Question,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_single_underscore_as_wildcard() {
    assert_eq!(
        lex_kinds("_ _name __"),
        vec![
            TokenKind::Underscore,
            TokenKind::Ident,
            TokenKind::Ident,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_dot_and_dotdot() {
    assert_eq!(
        lex_kinds(". .. 1..2"),
        vec![
            TokenKind::Dot,
            TokenKind::DotDot,
            TokenKind::Number,
            TokenKind::DotDot,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_unknown_ascii_as_error_not_eof() {
    assert_eq!(
        lex_token_data("@ x"),
        vec![
            (TokenKind::Error(LexErrorKind::InvalidChar), 0, 1),
            (TokenKind::Ident, 2, 3),
            (TokenKind::Eof, 3, 3),
        ]
    );
}

#[test]
fn lexes_unterminated_string_as_error() {
    assert_eq!(
        lex_token_data("\"unterminated"),
        vec![
            (TokenKind::Error(LexErrorKind::UnterminatedString), 0, 13,),
            (TokenKind::Eof, 13, 13),
        ]
    );
}

#[test]
fn skips_nested_block_comments() {
    assert_eq!(
        lex_kinds("x /* outer /* inner */ outer */ y"),
        vec![TokenKind::Ident, TokenKind::Ident, TokenKind::Eof]
    );
}

#[test]
fn lexes_unterminated_block_comment_as_error() {
    assert_eq!(
        lex_token_data("x /* unterminated"),
        vec![
            (TokenKind::Ident, 0, 1),
            (
                TokenKind::Error(LexErrorKind::UnterminatedBlockComment),
                2,
                17,
            ),
            (TokenKind::Eof, 17, 17),
        ]
    );
}

#[test]
fn asi_before_unterminated_block_comment_error() {
    assert_eq!(
        lex_token_data("x\n/* unterminated"),
        vec![
            (TokenKind::Ident, 0, 1),
            (TokenKind::Semi, 2, 2),
            (
                TokenKind::Error(LexErrorKind::UnterminatedBlockComment),
                2,
                17,
            ),
            (TokenKind::Eof, 17, 17),
        ]
    );
}

#[test]
fn lexes_number_tokens() {
    assert_eq!(
        lex_kinds("123 1_000 16r13FF 0r1011101 1.25 1. 1e10 1.2e-3"),
        vec![
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_number_spans_for_custom_radix_float_and_exponent() {
    assert_eq!(
        lex_token_data("16r13FF 1.25e-3"),
        vec![
            (TokenKind::Number, 0, 7),
            (TokenKind::Number, 8, 15),
            (TokenKind::Eof, 15, 15),
        ]
    );
}

#[test]
fn lexes_number_shapes_without_value_validation() {
    assert_eq!(
        lex_kinds("2r2 16r_ 1e 1e+ 1_"),
        vec![
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn numbers_may_contain_but_not_start_with_underscore() {
    assert_eq!(
        lex_kinds("_123 1_23"),
        vec![TokenKind::Ident, TokenKind::Number, TokenKind::Eof,]
    );
}

#[test]
fn asi_inserts_semicolon_after_terminator_across_newline() {
    assert_eq!(
        lex_kinds("x\n1\nreturn\nbreak\ncontinue\n"),
        vec![
            TokenKind::Ident,
            TokenKind::Semi,
            TokenKind::Number,
            TokenKind::Semi,
            TokenKind::KwReturn,
            TokenKind::Semi,
            TokenKind::KwBreak,
            TokenKind::Semi,
            TokenKind::KwContinue,
            TokenKind::Semi,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn asi_does_not_insert_after_non_terminator() {
    assert_eq!(
        lex_kinds("x +\ny"),
        vec![
            TokenKind::Ident,
            TokenKind::Plus,
            TokenKind::Ident,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn asi_does_not_insert_before_closer() {
    assert_eq!(
        lex_kinds("x\n) y\n] z\n}"),
        vec![
            TokenKind::Ident,
            TokenKind::RParen,
            TokenKind::Ident,
            TokenKind::RBracket,
            TokenKind::Ident,
            TokenKind::RBrace,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn asi_treats_newlines_in_comments_as_line_breaks() {
    assert_eq!(
        lex_kinds("x /*\n*/ y // comment\nz"),
        vec![
            TokenKind::Ident,
            TokenKind::Semi,
            TokenKind::Ident,
            TokenKind::Semi,
            TokenKind::Ident,
            TokenKind::Eof,
        ]
    );
}
