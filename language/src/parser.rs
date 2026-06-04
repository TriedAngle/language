use crate::index::{IdRange, RawRange};
use crate::{
    AssignOp, Ast, BinOp, BindingKind, Expr, ExprId, Field, GenericParam, GenericParamId, Interner,
    Literal, ParamId, Parameter, Pattern, Span, Stmt, StmtId, Symbol, Token, TokenKind, TypeExpr,
    TypeExprId, UnOp, lexer::Cursor,
};

pub struct Parser<'src> {
    cur: Cursor<'src>,
    src: &'src str,
    pub interner: Interner,
    pub ast: Ast,
    pub errors: Vec<ParseError>,
}

const ASSIGN_LBP: u8 = 2;
const ASSIGN_RBP: u8 = 1;

#[derive(Clone, Debug)]
pub struct ParseError {
    pub span: Span,
    pub kind: ParseErrorKind,
}

#[derive(Clone, Debug)]
pub enum ParseErrorKind {
    Expected(TokenKind),
    Unexpected(TokenKind),
    ExpectedExpr,
    ExpectedType,
}

fn bp(op: BinOp) -> (u8, u8) {
    match op {
        BinOp::Or => (3, 4),
        BinOp::And => (5, 6),
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => (7, 8),
        BinOp::Add | BinOp::Sub => (9, 10),
        BinOp::Mul | BinOp::Div | BinOp::Rem => (11, 12),
    }
}

fn infix(k: TokenKind) -> Option<BinOp> {
    use TokenKind::*;
    Some(match k {
        PipePipe => BinOp::Or,
        AmpAmp => BinOp::And,
        EqEq => BinOp::Eq,
        Ne => BinOp::Ne,
        Lt => BinOp::Lt,
        Le => BinOp::Le,
        Gt => BinOp::Gt,
        Ge => BinOp::Ge,
        Plus => BinOp::Add,
        Minus => BinOp::Sub,
        Star => BinOp::Mul,
        Slash => BinOp::Div,
        Percent => BinOp::Rem,
        _ => return None,
    })
}

enum AssignKind {
    Plain,
    Op(AssignOp),
}

impl<'src> Parser<'src> {
    pub fn new(src: &'src str) -> Self {
        Parser {
            cur: Cursor::new(src),
            src,
            interner: Interner::default(),
            ast: Ast::default(),
            errors: Vec::new(),
        }
    }

    fn peek(&self) -> TokenKind {
        self.cur.peek().kind
    }

    fn at(&self, k: TokenKind) -> bool {
        self.peek() == k
    }

    fn bump(&mut self) -> Token {
        self.cur.bump()
    }

    fn eat(&mut self, k: TokenKind) -> bool {
        if self.at(k) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, k: TokenKind) -> Token {
        if self.at(k) {
            self.bump()
        } else {
            let t = self.cur.peek();
            self.errors.push(ParseError {
                span: t.span,
                kind: ParseErrorKind::Expected(k),
            });
            t
        }
    }

    fn merge(&self, a: Span, b: Span) -> Span {
        Span {
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }

    fn lexeme(&self, t: Token) -> &'src str {
        let src: &'src str = self.src;
        &src[t.span.start as usize..t.span.end as usize]
    }

    fn symbol(&mut self, t: Token) -> Symbol {
        let s = self.lexeme(t);
        self.interner.intern(s)
    }

    fn int_value(&self, t: Token) -> i64 {
        self.lexeme(t).parse().unwrap_or(0)
    }

    pub fn parse_program(&mut self) -> Vec<StmtId> {
        let mut items = Vec::new();
        while !self.at(TokenKind::Eof) {
            if self.eat(TokenKind::Semi) {
                continue;
            }
            let s = self.statement();
            items.push(s);
            self.eat(TokenKind::Semi);
        }
        items
    }

    fn expr(&mut self) -> ExprId {
        self.expr_bp(0)
    }

    fn bottom_expr(&mut self) -> ExprId {
        let t = self.cur.peek();
        match t.kind {
            TokenKind::Number => {
                self.bump();
                let v = self.int_value(t);
                self.ast
                    .push_expr(Expr::Literal(Literal::Integer(v as i128)), t.span)
            }
            TokenKind::Ident => {
                self.bump();
                let s = self.symbol(t);
                self.ast.push_expr(Expr::Ident(s), t.span)
            }
            TokenKind::Str => {
                self.bump();
                let s = self.symbol(t);
                self.ast
                    .push_expr(Expr::Literal(Literal::String(s)), t.span)
            }
            TokenKind::LParen => {
                self.bump();
                let e = self.expr();
                // TODO: tuples
                self.expect(TokenKind::RParen);
                e
            }
            TokenKind::LBrace => self.block_expr(),
            TokenKind::Dot => self.record_expr(),
            // TokenKind::KwIf => self.if_expr(),
            TokenKind::KwFn => self.expr_fn(),
            // TokenKind::KwReturn => {
            //     self.bump();
            //     let v = if self.starts_expr() { Some(self.expr()) } else { None };
            //     self.ast.expr(Expr::Return(v), t.span)
            // }
            // TODO: KwMatch, KwWhile, KwLoop, KwBreak, KwContinue, true/false
            _ => {
                self.errors.push(ParseError {
                    span: t.span,
                    kind: ParseErrorKind::ExpectedExpr,
                });
                self.bump();
                self.ast.push_expr(Expr::Error, t.span)
            }
        }
    }

    fn statement(&mut self) -> StmtId {
        match self.peek() {
            TokenKind::KwLet => self.let_stmt(),
            TokenKind::KwFn => self.fn_stmt_or_expr_stmt(),
            TokenKind::KwStruct => self.struct_stmt(),

            _ => {
                let e = self.expr();
                let span = self.ast.expr_span(e);
                self.ast.push_stmt(Stmt::Expression(e), span)
            }
        }
    }

    fn let_stmt(&mut self) -> StmtId {
        let kw = self.expect(TokenKind::KwLet);

        let pattern = self.parse_pattern();

        let ty = if self.eat(TokenKind::Colon) {
            Some(self.parse_type())
        } else {
            None
        };
        let val = if self.eat(TokenKind::Eq) {
            Some(self.expr())
        } else {
            None
        };

        let kind = binding_kind(ty, val);
        self.ast.push_stmt(Stmt::Let { pattern, kind }, kw.span)
    }

    fn struct_stmt(&mut self) -> StmtId {
        let kw = self.expect(TokenKind::KwStruct);
        let nt = self.expect(TokenKind::Ident);
        let name = self.symbol(nt);
        let generics = self.parse_generic_params();
        let (fields, end) = self.parse_struct_fields();
        let span = self.merge(kw.span, end);

        self.ast.push_stmt(
            Stmt::Struct {
                name,
                generics,
                fields,
            },
            span,
        )
    }

    fn block_expr(&mut self) -> ExprId {
        let lbrace = self.expect(TokenKind::LBrace);
        let mut stmts = Vec::new();

        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            if self.eat(TokenKind::Semi) {
                continue;
            }

            stmts.push(self.statement());
            self.eat(TokenKind::Semi);
        }

        let rbrace = self.expect(TokenKind::RBrace);
        let span = self.merge(lbrace.span, rbrace.span);
        let stmts = self.ast.push_stmt_ids(&stmts);
        self.ast.push_expr(Expr::Body { stmts }, span)
    }

    fn record_expr(&mut self) -> ExprId {
        let dot = self.expect(TokenKind::Dot);
        let (fields, end) = self.parse_record_fields();
        let span = self.merge(dot.span, end);
        self.ast.push_expr(Expr::Record { fields }, span)
    }

    fn fn_stmt_or_expr_stmt(&mut self) -> StmtId {
        let kw = self.expect(TokenKind::KwFn);

        if self.at(TokenKind::Ident) {
            return self.fn_stmt_after_kw(kw);
        }

        let expr = self.expr_fn_after_kw(kw);
        let span = self.ast.expr_span(expr);
        self.ast.push_stmt(Stmt::Expression(expr), span)
    }

    fn fn_stmt_after_kw(&mut self, kw: Token) -> StmtId {
        let nt = self.expect(TokenKind::Ident);
        let name = self.symbol(nt);
        let generics = self.parse_generic_params();
        let parameters = self.parse_parameters();
        let ret = if self.eat(TokenKind::Arrow) {
            Some(self.parse_type())
        } else {
            None
        };
        let body = if self.at(TokenKind::LBrace) {
            Some(self.block_expr())
        } else {
            None
        };
        let span = body
            .map(|body| self.merge(kw.span, self.ast.expr_span(body)))
            .unwrap_or_else(|| self.merge(kw.span, nt.span));

        self.ast.push_stmt(
            Stmt::Function {
                name,
                generics,
                parameters,
                ret,
                body,
            },
            span,
        )
    }

    fn expr_fn(&mut self) -> ExprId {
        let kw = self.expect(TokenKind::KwFn);
        self.expr_fn_after_kw(kw)
    }

    fn expr_fn_after_kw(&mut self, kw: Token) -> ExprId {
        let generics = self.parse_generic_params();
        let parameters = self.parse_parameters();
        let ret = if self.eat(TokenKind::Arrow) {
            Some(self.parse_type())
        } else {
            None
        };
        let body = if self.at(TokenKind::LBrace) {
            self.block_expr()
        } else {
            let t = self.cur.peek();
            self.errors.push(ParseError {
                span: t.span,
                kind: ParseErrorKind::ExpectedExpr,
            });
            self.ast.push_expr(Expr::Error, t.span)
        };
        let span = self.merge(kw.span, self.ast.expr_span(body));
        self.ast.push_expr(
            Expr::Function {
                generics,
                parameters,
                ret,
                body,
            },
            span,
        )
    }

    fn parse_type(&mut self) -> TypeExprId {
        let t = self.cur.peek();
        match t.kind {
            TokenKind::Ident => {
                self.bump();
                let name = self.symbol(t);
                self.ast.push_type_expr(TypeExpr::Name(name), t.span)
            }
            TokenKind::KwStruct => self.parse_struct_type(),
            TokenKind::KwFn => self.parse_fn_type(),
            _ => {
                self.errors.push(ParseError {
                    span: t.span,
                    kind: ParseErrorKind::ExpectedType,
                });
                self.ast.push_type_expr(TypeExpr::Infer, t.span)
            }
        }
    }

    fn parse_struct_type(&mut self) -> TypeExprId {
        let kw = self.expect(TokenKind::KwStruct);
        let name = if self.at(TokenKind::Ident) {
            let nt = self.bump();
            Some(self.symbol(nt))
        } else {
            None
        };
        let generics = self.parse_generic_params();
        let (fields, end) = self.parse_struct_fields();
        let span = self.merge(kw.span, end);

        self.ast.push_type_expr(
            TypeExpr::Record {
                name,
                generics,
                fields,
            },
            span,
        )
    }

    fn parse_fn_type(&mut self) -> TypeExprId {
        let kw = self.expect(TokenKind::KwFn);
        let generics = self.parse_generic_params();
        let (params, rparen) = self.parse_parameters_with_span();
        let ret = if self.eat(TokenKind::Arrow) {
            Some(self.parse_type())
        } else {
            None
        };
        let end = ret
            .map(|ret| self.ast.type_expr_span(ret))
            .unwrap_or(rparen);
        let span = self.merge(kw.span, end);

        self.ast.push_type_expr(
            TypeExpr::Fn {
                generics,
                params,
                ret,
            },
            span,
        )
    }

    fn parse_struct_fields(&mut self) -> (RawRange<crate::FieldId>, Span) {
        if !self.at(TokenKind::LBrace) {
            let t = self.expect(TokenKind::LBrace);
            return (self.ast.push_fields(Vec::new()), t.span);
        }

        self.bump();
        let mut fields = Vec::new();

        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            if self.eat(TokenKind::Comma) || self.eat(TokenKind::Semi) {
                continue;
            }

            fields.push(self.parse_field());

            if self.eat(TokenKind::Comma) || self.eat(TokenKind::Semi) {
                continue;
            }
        }

        let rbrace = self.expect(TokenKind::RBrace);
        (self.ast.push_fields(fields), rbrace.span)
    }

    fn parse_field(&mut self) -> Field {
        let t = self.cur.peek();
        let (name, mut span) = match t.kind {
            TokenKind::Ident => {
                self.bump();
                (self.symbol(t), t.span)
            }
            _ => {
                self.errors.push(ParseError {
                    span: t.span,
                    kind: ParseErrorKind::Expected(TokenKind::Ident),
                });
                self.bump();
                (self.interner.intern("_"), t.span)
            }
        };

        let ty = if self.eat(TokenKind::Colon) {
            let ty = self.parse_type();
            span = self.merge(span, self.ast.type_expr_span(ty));
            Some(ty)
        } else {
            None
        };

        let default = if self.eat(TokenKind::Eq) {
            let default = self.expr();
            span = self.merge(span, self.ast.expr_span(default));
            Some(default)
        } else {
            None
        };

        let kind = binding_kind(ty, default);

        Field { span, name, kind }
    }

    fn parse_record_fields(&mut self) -> (RawRange<crate::FieldId>, Span) {
        if !self.at(TokenKind::LBrace) {
            let t = self.expect(TokenKind::LBrace);
            return (self.ast.push_fields(Vec::new()), t.span);
        }

        self.bump();
        let mut fields = Vec::new();

        while !self.at(TokenKind::RBrace) && !self.at(TokenKind::Eof) {
            if self.eat(TokenKind::Comma) || self.eat(TokenKind::Semi) {
                continue;
            }

            fields.push(self.parse_record_field());

            if self.eat(TokenKind::Comma) || self.eat(TokenKind::Semi) {
                continue;
            }
        }

        let rbrace = self.expect(TokenKind::RBrace);
        (self.ast.push_fields(fields), rbrace.span)
    }

    fn parse_record_field(&mut self) -> Field {
        let t = self.cur.peek();
        let (name, mut span) = match t.kind {
            TokenKind::Ident => {
                self.bump();
                (self.symbol(t), t.span)
            }
            _ => {
                self.errors.push(ParseError {
                    span: t.span,
                    kind: ParseErrorKind::Expected(TokenKind::Ident),
                });
                self.bump();
                (self.interner.intern("_"), t.span)
            }
        };

        let value = if self.eat(TokenKind::Colon) {
            let value = self.expr();
            span = self.merge(span, self.ast.expr_span(value));
            value
        } else {
            self.ast.push_expr(Expr::Ident(name), span)
        };

        Field {
            span,
            name,
            kind: BindingKind::Value(value),
        }
    }

    fn parse_parameters(&mut self) -> RawRange<ParamId> {
        self.parse_parameters_with_span().0
    }

    fn parse_parameters_with_span(&mut self) -> (RawRange<ParamId>, Span) {
        self.expect(TokenKind::LParen);
        let mut params = Vec::new();

        if !self.at(TokenKind::RParen) {
            loop {
                if self.at(TokenKind::Eof) {
                    break;
                }

                params.push(self.parse_parameter());

                if !self.eat(TokenKind::Comma) {
                    break;
                }
                if self.at(TokenKind::RParen) {
                    break;
                }
            }
        }

        let rparen = self.expect(TokenKind::RParen);
        (self.ast.push_params(params), rparen.span)
    }

    fn parse_generic_params(&mut self) -> RawRange<GenericParamId> {
        let mut params = Vec::new();

        if !self.eat(TokenKind::LBracket) {
            return self.ast.push_generic_params(params);
        }

        if !self.at(TokenKind::RBracket) {
            loop {
                if self.at(TokenKind::Eof) {
                    break;
                }

                params.push(self.parse_generic_param());

                if !self.eat(TokenKind::Comma) {
                    break;
                }
                if self.at(TokenKind::RBracket) {
                    break;
                }
            }
        }

        self.expect(TokenKind::RBracket);
        self.ast.push_generic_params(params)
    }

    fn parse_generic_param(&mut self) -> GenericParam {
        let t = self.cur.peek();
        let (name, mut span) = match t.kind {
            TokenKind::Ident => {
                self.bump();
                (self.symbol(t), t.span)
            }
            _ => {
                self.errors.push(ParseError {
                    span: t.span,
                    kind: ParseErrorKind::Expected(TokenKind::Ident),
                });
                self.bump();
                (self.interner.intern("_"), t.span)
            }
        };

        let ty = if self.eat(TokenKind::Colon) {
            let ty = self.parse_type();
            span = self.merge(span, self.ast.type_expr_span(ty));
            Some(ty)
        } else {
            None
        };

        GenericParam { span, name, ty }
    }

    fn parse_parameter(&mut self) -> Parameter {
        let pattern = self.parse_pattern();
        let mut span = self.ast.pattern_span(pattern);

        let ty = if self.eat(TokenKind::Colon) {
            let ty = self.parse_type();
            span = self.merge(span, self.ast.type_expr_span(ty));
            Some(ty)
        } else {
            None
        };

        let default = if self.eat(TokenKind::Eq) {
            let default = self.expr();
            span = self.merge(span, self.ast.expr_span(default));
            Some(default)
        } else {
            None
        };

        Parameter {
            span,
            pattern,
            kind: binding_kind(ty, default),
        }
    }

    fn parse_pattern(&mut self) -> crate::PatternId {
        let t = self.cur.peek();
        match t.kind {
            TokenKind::Ident => {
                self.bump();
                let name = self.symbol(t);
                self.ast.push_pattern(Pattern::Bind(name), t.span)
            }
            TokenKind::Underscore => {
                self.bump();
                self.ast.push_pattern(Pattern::Wildcard, t.span)
            }
            _ => {
                self.errors.push(ParseError {
                    span: t.span,
                    kind: ParseErrorKind::Expected(TokenKind::Ident),
                });
                self.bump();
                self.ast.push_pattern(Pattern::Wildcard, t.span)
            }
        }
    }

    fn prefix_expr(&mut self) -> ExprId {
        let t = self.cur.peek();
        match t.kind {
            TokenKind::Minus => {
                self.bump();
                let rhs = self.prefix_expr();
                let span = self.merge(t.span, self.ast.expr_span(rhs));
                self.ast.push_expr(Expr::Unary { op: UnOp::Neg, rhs }, span)
            }
            TokenKind::Bang => {
                self.bump();
                let rhs = self.prefix_expr();
                let span = self.merge(t.span, self.ast.expr_span(rhs));
                self.ast.push_expr(Expr::Unary { op: UnOp::Not, rhs }, span)
            }
            _ => self.postfix_expr(),
        }
    }

    fn postfix_expr(&mut self) -> ExprId {
        let mut node = self.bottom_expr();

        // postfix loop — tighter than any binary operator
        loop {
            match self.peek() {
                TokenKind::Dot => {
                    self.bump();
                    let nt = self.expect(TokenKind::Ident);
                    let field = self.symbol(nt);
                    let span = self.merge(self.ast.expr_span(node), nt.span);
                    node = self.ast.push_expr(Expr::Field { base: node, field }, span);
                }
                TokenKind::LParen => {
                    let start = self.ast.expr_span(node);
                    self.bump();
                    let (args, rparen) = self.call_args();
                    let span = self.merge(start, rparen);
                    node = self.ast.push_expr(Expr::Call { callee: node, args }, span);
                }
                _ => break,
            }
        }
        node
    }

    fn call_args(&mut self) -> (IdRange<ExprId>, Span) {
        let mut args = Vec::new();

        if !self.at(TokenKind::RParen) {
            loop {
                if self.at(TokenKind::Eof) {
                    break;
                }

                args.push(self.expr());

                if !self.eat(TokenKind::Comma) {
                    break;
                }
                if self.at(TokenKind::RParen) {
                    break;
                }
            }
        }

        let rparen = self.expect(TokenKind::RParen);
        (self.ast.push_expr_ids(&args), rparen.span)
    }

    fn expr_bp(&mut self, min_bp: u8) -> ExprId {
        let mut lhs = self.prefix_expr();
        loop {
            let k = self.peek();

            if let Some(kind) = assign_kind(k) {
                if ASSIGN_LBP < min_bp {
                    break;
                }
                self.bump();
                let value = self.expr_bp(ASSIGN_RBP);
                let span = self.merge(self.ast.expr_span(lhs), self.ast.expr_span(value));
                lhs = match kind {
                    AssignKind::Plain => self
                        .ast
                        .push_expr(Expr::Assign { target: lhs, value }, span),
                    AssignKind::Op(op) => self.ast.push_expr(
                        Expr::AssignOp {
                            op,
                            target: lhs,
                            value,
                        },
                        span,
                    ),
                };
                continue;
            }

            // ordinary binary operators, via the bp table
            let op = match infix(k) {
                Some(op) => op,
                None => break,
            };
            let (l, r) = bp(op);
            if l < min_bp {
                break;
            }
            self.bump();
            let rhs = self.expr_bp(r);
            let span = self.merge(self.ast.expr_span(lhs), self.ast.expr_span(rhs));
            lhs = self.ast.push_expr(Expr::Binary { op, lhs, rhs }, span);
        }
        lhs
    }
}

fn assign_kind(k: TokenKind) -> Option<AssignKind> {
    use TokenKind::*;
    Some(match k {
        Eq => AssignKind::Plain,
        PlusEq => AssignKind::Op(AssignOp::Add),
        MinusEq => AssignKind::Op(AssignOp::Sub),
        StarEq => AssignKind::Op(AssignOp::Mul),
        SlashEq => AssignKind::Op(AssignOp::Div),
        _ => return None,
    })
}

fn binding_kind(ty: Option<TypeExprId>, value: Option<ExprId>) -> BindingKind {
    match (ty, value) {
        (Some(ty), Some(value)) => BindingKind::Full { ty, value },
        (Some(ty), None) => BindingKind::Type(ty),
        (None, Some(value)) => BindingKind::Value(value),
        (None, None) => BindingKind::Name,
    }
}
