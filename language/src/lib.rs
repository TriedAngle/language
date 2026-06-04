use std::collections::HashMap;

use crate::index::{IdRange, IndexPool, IndexVec, RawRange};

pub mod index;
pub mod lexer;
pub mod parser;

make_index!(ExprId);
make_index!(StmtId);
make_index!(PatternId);
make_index!(TypeExprId);
make_index!(MatchArmId);
make_index!(FieldId);
make_index!(ParamId);
make_index!(VariantId);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Symbol(u32);

#[derive(Debug, Copy, Clone)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum Expr {
    Literal(Literal),
    Unary {
        op: UnOp,
        rhs: ExprId,
    },
    Binary {
        op: BinOp,
        lhs: ExprId,
        rhs: ExprId,
    },
    Assign {
        target: ExprId,
        value: ExprId,
    },
    AssignOp {
        op: AssignOp,
        target: ExprId,
        value: ExprId,
    },
    Call {
        callee: ExprId,
        args: IdRange<ExprId>,
    },
    Field {
        base: ExprId,
        field: Symbol,
    },
    If {
        cond: ExprId,
        then_br: ExprId,
        else_br: Option<ExprId>,
    },
    While {
        cond: ExprId,
        body: ExprId,
    },
    Match {
        scrutinee: ExprId,
        arms: IdRange<MatchArmId>,
    },
    Break(Option<ExprId>),
    Continue,
    Return(Option<ExprId>),
    Body {
        stmts: IdRange<StmtId>,
    },
    Function {
        parameters: RawRange<ParamId>,
        ret: Option<TypeExprId>,
        body: ExprId,
    },
    Error,
}

#[derive(Debug, Clone, Copy)]
pub enum Stmt {
    Expression(ExprId),
    Let {
        pattern: PatternId,
        kind: LetKind,
    },
    Function {
        name: Symbol,
        parameters: RawRange<ParamId>,
        ret: Option<TypeExprId>,
        body: Option<ExprId>,
    },
    Struct {
        name: Symbol,
        fields: RawRange<FieldId>,
    },
    Enum {
        name: Symbol,
        variants: RawRange<VariantId>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum TypeExpr {
    Name(Symbol),
    Infer,
    Pointer(TypeExprId),
    Tuple(IdRange<TypeExprId>),
    Join {
        lhs: TypeExprId,
        rhs: TypeExprId,
    },
    Record {
        name: Option<Symbol>,
        fields: IdRange<FieldId>,
    },
    Fn {
        params: IdRange<TypeExprId>,
        ret: IdRange<TypeExprId>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum Pattern {
    Wildcard,
    Bind(Symbol),
    Tuple(IdRange<PatternId>),
}

#[derive(Debug, Clone, Copy)]
pub enum LetKind {
    Type(TypeExprId),
    Value(ExprId),
    Full { ty: TypeExprId, value: ExprId },
}

#[rustfmt::skip]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AssignOp {
    Add, Sub, Mul, Div, Rem,
}

#[rustfmt::skip]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Eq, Ne, Lt, Le, Gt, Ge,
}

#[rustfmt::skip]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UnOp {
    Neg, Not,
}

#[derive(Debug, Clone, Copy)]
pub enum Literal {
    Integer(i128),
    Float(f64),
    String(Symbol),
}

#[derive(Debug, Clone, Copy)]
pub struct EnumVariant {
    pub name: Symbol,
    pub ty: Option<TypeExprId>,
}

#[derive(Debug, Clone, Copy)]
pub struct Parameter {
    pub span: Span,
    pub name: PatternId,
    pub ty: Option<TypeExprId>,
    pub default: Option<ExprId>,
}

#[derive(Copy, Clone, Debug)]
pub struct Field {
    pub span: Span,
    pub name: Symbol,
    pub ty: TypeExprId,
    pub default: Option<ExprId>,
}

#[derive(Debug, Copy, Clone)]
pub struct MatchArm {
    pub span: Span,
    pub pat: PatternId,
    pub guard: Option<ExprId>,
    pub body: ExprId,
}

#[derive(Default)]
pub struct Interner {
    map: HashMap<String, u32>,
    strs: Vec<String>,
}

impl Interner {
    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&id) = self.map.get(s) {
            return Symbol(id);
        }
        let id = self.strs.len() as u32;
        self.strs.push(s.to_owned());
        self.map.insert(s.to_owned(), id);
        Symbol(id)
    }
    pub fn resolve(&self, s: Symbol) -> &str {
        &self.strs[s.0 as usize]
    }
}

#[derive(Debug, Default)]
pub struct Ast {
    exprs: IndexPool<ExprId, Expr>,
    expr_spans: IndexVec<ExprId, Span>,
    stmts: IndexPool<StmtId, Stmt>,
    stmt_spans: IndexVec<StmtId, Span>,
    patterns: IndexPool<PatternId, Pattern>,
    pattern_spans: IndexVec<PatternId, Span>,
    type_exprs: IndexPool<TypeExprId, TypeExpr>,
    type_expr_spans: IndexVec<TypeExprId, Span>,
    match_arms: IndexPool<MatchArmId, MatchArm>,
    fields: IndexPool<FieldId, Field>,
    params: IndexPool<ParamId, Parameter>,
    variants: IndexPool<VariantId, EnumVariant>,
}

impl Ast {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_expr(&mut self, expr: Expr, span: Span) -> ExprId {
        let id = self.exprs.alloc(expr);
        let span_id = self.expr_spans.push(span);
        debug_assert_eq!(id, span_id);
        id
    }

    pub fn expr(&self, id: ExprId) -> &Expr {
        self.exprs.get(id)
    }

    pub fn expr_mut(&mut self, id: ExprId) -> &mut Expr {
        self.exprs.get_mut(id)
    }

    pub fn expr_span(&self, id: ExprId) -> Span {
        self.expr_spans[id]
    }

    pub fn push_expr_ids(&mut self, ids: &[ExprId]) -> IdRange<ExprId> {
        self.exprs.alloc_range(ids)
    }

    pub fn expr_ids(&self, range: IdRange<ExprId>) -> &[ExprId] {
        self.exprs.range(range)
    }

    pub fn push_stmt(&mut self, stmt: Stmt, span: Span) -> StmtId {
        let id = self.stmts.alloc(stmt);
        let span_id = self.stmt_spans.push(span);
        debug_assert_eq!(id, span_id);
        id
    }

    pub fn stmt(&self, id: StmtId) -> &Stmt {
        self.stmts.get(id)
    }

    pub fn stmt_mut(&mut self, id: StmtId) -> &mut Stmt {
        self.stmts.get_mut(id)
    }

    pub fn stmt_span(&self, id: StmtId) -> Span {
        self.stmt_spans[id]
    }

    pub fn push_stmt_ids(&mut self, ids: &[StmtId]) -> IdRange<StmtId> {
        self.stmts.alloc_range(ids)
    }

    pub fn stmt_ids(&self, range: IdRange<StmtId>) -> &[StmtId] {
        self.stmts.range(range)
    }

    pub fn push_pattern(&mut self, pattern: Pattern, span: Span) -> PatternId {
        let id = self.patterns.alloc(pattern);
        let span_id = self.pattern_spans.push(span);
        debug_assert_eq!(id, span_id);
        id
    }

    pub fn pattern(&self, id: PatternId) -> &Pattern {
        self.patterns.get(id)
    }

    pub fn pattern_mut(&mut self, id: PatternId) -> &mut Pattern {
        self.patterns.get_mut(id)
    }

    pub fn pattern_span(&self, id: PatternId) -> Span {
        self.pattern_spans[id]
    }

    pub fn push_pattern_ids(&mut self, ids: &[PatternId]) -> IdRange<PatternId> {
        self.patterns.alloc_range(ids)
    }

    pub fn pattern_ids(&self, range: IdRange<PatternId>) -> &[PatternId] {
        self.patterns.range(range)
    }

    pub fn push_type_expr(&mut self, type_expr: TypeExpr, span: Span) -> TypeExprId {
        let id = self.type_exprs.alloc(type_expr);
        let span_id = self.type_expr_spans.push(span);
        debug_assert_eq!(id, span_id);
        id
    }

    pub fn type_expr(&self, id: TypeExprId) -> &TypeExpr {
        self.type_exprs.get(id)
    }

    pub fn type_expr_mut(&mut self, id: TypeExprId) -> &mut TypeExpr {
        self.type_exprs.get_mut(id)
    }

    pub fn type_expr_span(&self, id: TypeExprId) -> Span {
        self.type_expr_spans[id]
    }

    pub fn push_type_expr_ids(&mut self, ids: &[TypeExprId]) -> IdRange<TypeExprId> {
        self.type_exprs.alloc_range(ids)
    }

    pub fn type_expr_ids(&self, range: IdRange<TypeExprId>) -> &[TypeExprId] {
        self.type_exprs.range(range)
    }

    pub fn push_match_arm(&mut self, arm: MatchArm) -> MatchArmId {
        self.match_arms.alloc(arm)
    }

    pub fn match_arm(&self, id: MatchArmId) -> &MatchArm {
        self.match_arms.get(id)
    }

    pub fn match_arm_mut(&mut self, id: MatchArmId) -> &mut MatchArm {
        self.match_arms.get_mut(id)
    }

    pub fn push_match_arm_ids(&mut self, ids: &[MatchArmId]) -> IdRange<MatchArmId> {
        self.match_arms.alloc_range(ids)
    }

    pub fn match_arm_ids(&self, range: IdRange<MatchArmId>) -> &[MatchArmId] {
        self.match_arms.range(range)
    }

    pub fn push_field(&mut self, field: Field) -> FieldId {
        self.fields.alloc(field)
    }

    pub fn push_fields(&mut self, fields: impl IntoIterator<Item = Field>) -> RawRange<FieldId> {
        self.fields.alloc_raw_range(fields)
    }

    pub fn field(&self, id: FieldId) -> &Field {
        self.fields.get(id)
    }

    pub fn field_mut(&mut self, id: FieldId) -> &mut Field {
        self.fields.get_mut(id)
    }

    pub fn fields(&self, range: RawRange<FieldId>) -> &[Field] {
        self.fields.raw_range(range)
    }

    pub fn push_field_ids(&mut self, ids: &[FieldId]) -> IdRange<FieldId> {
        self.fields.alloc_range(ids)
    }

    pub fn field_ids(&self, range: IdRange<FieldId>) -> &[FieldId] {
        self.fields.range(range)
    }

    pub fn push_param(&mut self, param: Parameter) -> ParamId {
        self.params.alloc(param)
    }

    pub fn push_params(
        &mut self,
        params: impl IntoIterator<Item = Parameter>,
    ) -> RawRange<ParamId> {
        self.params.alloc_raw_range(params)
    }

    pub fn param(&self, id: ParamId) -> &Parameter {
        self.params.get(id)
    }

    pub fn param_mut(&mut self, id: ParamId) -> &mut Parameter {
        self.params.get_mut(id)
    }

    pub fn params(&self, range: RawRange<ParamId>) -> &[Parameter] {
        self.params.raw_range(range)
    }

    pub fn push_variant(&mut self, variant: EnumVariant) -> VariantId {
        self.variants.alloc(variant)
    }

    pub fn push_variants(
        &mut self,
        variants: impl IntoIterator<Item = EnumVariant>,
    ) -> RawRange<VariantId> {
        self.variants.alloc_raw_range(variants)
    }

    pub fn variant(&self, id: VariantId) -> &EnumVariant {
        self.variants.get(id)
    }

    pub fn variant_mut(&mut self, id: VariantId) -> &mut EnumVariant {
        self.variants.get_mut(id)
    }

    pub fn variants(&self, range: RawRange<VariantId>) -> &[EnumVariant] {
        self.variants.raw_range(range)
    }
}

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
