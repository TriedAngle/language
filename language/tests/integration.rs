use language::lexer::Lexer;
use language::parser::Parser;
use language::{BindingKind, Expr, LexErrorKind, Stmt, TokenKind, TypeExpr};

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
            TokenKind::Dot,
            TokenKind::Number,
            TokenKind::Number,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn lexes_trailing_dot_number_as_number_then_dot() {
    assert_eq!(
        lex_kinds("1. .{"),
        vec![
            TokenKind::Number,
            TokenKind::Dot,
            TokenKind::Dot,
            TokenKind::LBrace,
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

#[test]
fn parser_handles_nested_let_block_sample() {
    let src = r#"let x = {
        let val1 = 20*(2+3*3)
            val1
        }
        let res = x+2 * 3
    "#;

    let mut parser = Parser::new(src);
    let stmts = parser.parse_program();

    assert!(parser.errors.is_empty(), "{:?}", parser.errors);
    assert_eq!(stmts.len(), 2);

    let x_value = match *parser.ast.stmt(stmts[0]) {
        Stmt::Let {
            kind: BindingKind::Value(value),
            ..
        } => value,
        stmt => panic!("expected first let statement, got {stmt:?}"),
    };

    let body = match *parser.ast.expr(x_value) {
        Expr::Body { stmts } => stmts,
        expr => panic!("expected block body, got {expr:?}"),
    };
    let body_stmts = parser.ast.stmt_ids(body);

    assert_eq!(body_stmts.len(), 2);
    assert!(matches!(*parser.ast.stmt(body_stmts[0]), Stmt::Let { .. }));
    assert!(matches!(
        *parser.ast.stmt(body_stmts[1]),
        Stmt::Expression(_)
    ));

    let res_value = match *parser.ast.stmt(stmts[1]) {
        Stmt::Let {
            kind: BindingKind::Value(value),
            ..
        } => value,
        stmt => panic!("expected second let statement, got {stmt:?}"),
    };
    assert!(matches!(
        *parser.ast.expr(res_value),
        Expr::Binary {
            op: _,
            lhs: _,
            rhs: _
        }
    ));
}

#[test]
fn parser_handles_positional_and_named_call_args() {
    let src = "let a = f(1, x + 2)\nlet b = f(left = 1, right = 2)";

    let mut parser = Parser::new(src);
    let stmts = parser.parse_program();

    assert!(parser.errors.is_empty(), "{:?}", parser.errors);
    assert_eq!(stmts.len(), 2);

    let first_value = match *parser.ast.stmt(stmts[0]) {
        Stmt::Let {
            kind: BindingKind::Value(value),
            ..
        } => value,
        stmt => panic!("expected first let statement, got {stmt:?}"),
    };
    let first_args = match *parser.ast.expr(first_value) {
        Expr::Call { args, .. } => parser.ast.expr_ids(args),
        expr => panic!("expected first call, got {expr:?}"),
    };
    assert_eq!(first_args.len(), 2);

    let second_value = match *parser.ast.stmt(stmts[1]) {
        Stmt::Let {
            kind: BindingKind::Value(value),
            ..
        } => value,
        stmt => panic!("expected second let statement, got {stmt:?}"),
    };
    let second_args = match *parser.ast.expr(second_value) {
        Expr::Call { args, .. } => parser.ast.expr_ids(args),
        expr => panic!("expected second call, got {expr:?}"),
    };

    assert_eq!(second_args.len(), 2);
    assert!(matches!(
        *parser.ast.expr(second_args[0]),
        Expr::Assign {
            target: _,
            value: _
        }
    ));
    assert!(matches!(
        *parser.ast.expr(second_args[1]),
        Expr::Assign {
            target: _,
            value: _
        }
    ));
}

#[test]
fn parser_handles_function_expression() {
    let src = "let f = fn[T: Type, U](x: Int, y = 2, z: Int = 3,) -> Int { x + y + z }";

    let mut parser = Parser::new(src);
    let stmts = parser.parse_program();

    assert!(parser.errors.is_empty(), "{:?}", parser.errors);
    assert_eq!(stmts.len(), 1);

    let value = match *parser.ast.stmt(stmts[0]) {
        Stmt::Let {
            kind: BindingKind::Value(value),
            ..
        } => value,
        stmt => panic!("expected let statement, got {stmt:?}"),
    };

    let params = match *parser.ast.expr(value) {
        Expr::Function {
            generics,
            parameters,
            ret,
            ..
        } => {
            assert_eq!(parser.ast.generic_params(generics).len(), 2);
            assert!(ret.is_some());
            parser.ast.params(parameters)
        }
        expr => panic!("expected function expression, got {expr:?}"),
    };

    assert_eq!(params.len(), 3);
    assert!(matches!(params[0].kind, BindingKind::Type(_)));
    assert!(matches!(params[1].kind, BindingKind::Value(_)));
    assert!(matches!(params[2].kind, BindingKind::Full { .. }));
}

#[test]
fn parser_handles_named_function_statement() {
    let src = "fn hello[T: Type, U](x: Int, y = 2, z: Int = 3,) -> Int { x + y + z }";

    let mut parser = Parser::new(src);
    let stmts = parser.parse_program();

    assert!(parser.errors.is_empty(), "{:?}", parser.errors);
    assert_eq!(stmts.len(), 1);

    let params = match *parser.ast.stmt(stmts[0]) {
        Stmt::Function {
            generics,
            parameters,
            ret,
            body,
            ..
        } => {
            assert_eq!(parser.ast.generic_params(generics).len(), 2);
            assert!(ret.is_some());
            assert!(body.is_some());
            parser.ast.params(parameters)
        }
        stmt => panic!("expected function statement, got {stmt:?}"),
    };

    assert_eq!(params.len(), 3);
    assert!(matches!(params[0].kind, BindingKind::Type(_)));
    assert!(matches!(params[1].kind, BindingKind::Value(_)));
    assert!(matches!(params[2].kind, BindingKind::Full { .. }));
}

#[test]
fn parser_handles_name_only_bindings() {
    let src = "let x\nfn id(x) { x }\nstruct Names { value }";

    let mut parser = Parser::new(src);
    let stmts = parser.parse_program();

    assert!(parser.errors.is_empty(), "{:?}", parser.errors);
    assert_eq!(stmts.len(), 3);

    assert!(matches!(
        *parser.ast.stmt(stmts[0]),
        Stmt::Let {
            kind: BindingKind::Name,
            ..
        }
    ));

    let params = match *parser.ast.stmt(stmts[1]) {
        Stmt::Function { parameters, .. } => parser.ast.params(parameters),
        stmt => panic!("expected function statement, got {stmt:?}"),
    };
    assert_eq!(params.len(), 1);
    assert!(matches!(params[0].kind, BindingKind::Name));

    let fields = match *parser.ast.stmt(stmts[2]) {
        Stmt::Struct { fields, .. } => parser.ast.fields(fields),
        stmt => panic!("expected struct statement, got {stmt:?}"),
    };
    assert_eq!(fields.len(), 1);
    assert!(matches!(fields[0].kind, BindingKind::Name));
}

#[test]
fn parser_handles_generic_struct_with_nested_anonymous_struct_type() {
    let src = r#"struct Test[T: Type] {
  table: struct {
    hello: fn(i32) -> i32,
    value: i32
  }
}"#;

    let mut parser = Parser::new(src);
    let stmts = parser.parse_program();

    assert!(parser.errors.is_empty(), "{:?}", parser.errors);
    assert_eq!(stmts.len(), 1);

    let fields = match *parser.ast.stmt(stmts[0]) {
        Stmt::Struct {
            generics, fields, ..
        } => {
            assert_eq!(parser.ast.generic_params(generics).len(), 1);
            parser.ast.fields(fields)
        }
        stmt => panic!("expected struct statement, got {stmt:?}"),
    };

    assert_eq!(fields.len(), 1);
    let table_ty = match fields[0].kind {
        BindingKind::Type(ty) => ty,
        kind => panic!("expected typed table field, got {kind:?}"),
    };

    let nested_fields = match *parser.ast.type_expr(table_ty) {
        TypeExpr::Record { fields, .. } => parser.ast.fields(fields),
        expr => panic!("expected anonymous struct type, got {expr:?}"),
    };

    assert_eq!(nested_fields.len(), 2);

    let hello_ty = match nested_fields[0].kind {
        BindingKind::Type(ty) => ty,
        kind => panic!("expected typed hello field, got {kind:?}"),
    };

    match *parser.ast.type_expr(hello_ty) {
        TypeExpr::Fn { params, ret, .. } => {
            assert_eq!(parser.ast.params(params).len(), 1);
            assert!(ret.is_some());
        }
        expr => panic!("expected fn type, got {expr:?}"),
    }

    assert!(matches!(nested_fields[1].kind, BindingKind::Type(_)));
}

#[test]
fn parser_handles_anonymous_record_expression() {
    let src = r#"let value = .{
  table: .{ hello: f, value: 1 },
  count: 2,
}"#;

    let mut parser = Parser::new(src);
    let stmts = parser.parse_program();

    assert!(parser.errors.is_empty(), "{:?}", parser.errors);
    assert_eq!(stmts.len(), 1);

    let value = match *parser.ast.stmt(stmts[0]) {
        Stmt::Let {
            kind: BindingKind::Value(value),
            ..
        } => value,
        stmt => panic!("expected let statement, got {stmt:?}"),
    };

    let fields = match *parser.ast.expr(value) {
        Expr::Record { fields } => parser.ast.fields(fields),
        expr => panic!("expected anonymous record expression, got {expr:?}"),
    };

    assert_eq!(fields.len(), 2);

    let nested = match fields[0].kind {
        BindingKind::Value(value) => value,
        kind => panic!("expected record field value, got {kind:?}"),
    };

    let nested_fields = match *parser.ast.expr(nested) {
        Expr::Record { fields } => parser.ast.fields(fields),
        expr => panic!("expected nested anonymous record expression, got {expr:?}"),
    };

    assert_eq!(nested_fields.len(), 2);
    assert!(matches!(fields[1].kind, BindingKind::Value(_)));
}

#[test]
fn parser_handles_anonymous_record_shorthand_fields() {
    let src = "let value = .{ hello, count: 2 }";

    let mut parser = Parser::new(src);
    let stmts = parser.parse_program();

    assert!(parser.errors.is_empty(), "{:?}", parser.errors);
    assert_eq!(stmts.len(), 1);

    let value = match *parser.ast.stmt(stmts[0]) {
        Stmt::Let {
            kind: BindingKind::Value(value),
            ..
        } => value,
        stmt => panic!("expected let statement, got {stmt:?}"),
    };

    let fields = match *parser.ast.expr(value) {
        Expr::Record { fields } => parser.ast.fields(fields),
        expr => panic!("expected anonymous record expression, got {expr:?}"),
    };

    assert_eq!(fields.len(), 2);

    let shorthand_value = match fields[0].kind {
        BindingKind::Value(value) => value,
        kind => panic!("expected shorthand field value, got {kind:?}"),
    };

    assert!(matches!(*parser.ast.expr(shorthand_value), Expr::Ident(_)));
    assert!(matches!(fields[1].kind, BindingKind::Value(_)));
}
