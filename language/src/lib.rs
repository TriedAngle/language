use crate::index::{IdRange, IndexPool, RawRange};

pub mod index;

make_index!(ExprId);
make_index!(StmtId);
make_index!(PatternId);
make_index!(TypeExprId);
make_index!(MatchArmId);
make_index!(FieldId);
make_index!(ParamId);
make_index!(VariantId);

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Symbol(u32);

#[derive(Debug, Clone, Copy)]
pub enum Expr {
    Body {
        literal: LiteralKind,
        stmts: IdRange<StmtId>,
    },
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
        ret: TypeExprId,
        body: ExprId,
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
pub enum TypeExpr {}

#[derive(Debug, Clone, Copy)]
pub enum Pattern {
    Wildcard,
    Bind(Symbol),
}

#[derive(Debug, Clone, Copy)]
pub enum LetKind {
    Type(TypeExprId),
    Value(ExprId),
    Full { ty: TypeExprId, value: ExprId },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}
 
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum UnOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy)]
pub enum LiteralKind {
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
