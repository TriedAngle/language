use crate::index::{IdRange, IndexPool, RawRange};

pub mod index;
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
    pub name: PatternId,
    pub ty: Option<TypeExprId>,
    pub default: Option<ExprId>,
}

#[derive(Copy, Clone, Debug)]
pub struct Field {
    pub name: Symbol,
    pub ty: TypeExprId,
    pub default: Option<ExprId>,
}

#[derive(Debug, Copy, Clone)]
pub struct MatchArm {
    pub pat: PatternId,
    pub guard: Option<ExprId>,
    pub body: ExprId,
}

#[derive(Debug)]
pub struct Ast {
    exprs: IndexPool<ExprId, Expr>,
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
