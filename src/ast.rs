use std::collections::BTreeMap;



#[derive(Debug, Clone)]
pub struct Var ( pub String, pub usize );  // label, index


#[derive(Debug, Clone)]
pub enum Num {
    Bool(bool),
    Natural(u64),
    Integer(i64),
    Double(f64),
}

#[derive(Debug, Clone)]
pub enum Expr {

    // Some
    Some(Box<Expr>),

    BaseType(BaseType),
    Text(Vec<(String, Option<Expr>)>),
    TextLit(String),
    Num(Num),
    Builtin(Builtin),
    // let x : t = r in e
    Let(String, Box<Option<Expr>>, Box<Expr>, Box<Expr>),
    // Record
    RecordType(BTreeMap<String, Expr>),
    Record(BTreeMap<String, Expr>),
    // List
    List(Vec<Expr>),
    ListType(Box<Expr>),

    Var(Var),
    // \(x : A) -> b
    Select(Box<Expr>, String),
    Lambda(String, Box<Expr>, Box<Expr>),  // arg-name, arg-type, expr
    FnType(Box<Expr>, Box<Expr>),

    // Operations
    Op(Op),

    // x : t
    Annot(Box<Expr>, Box<Expr>),
    // assert : x
    Assert(Box<Expr>),
    Import(Import),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Import {
    Local(String),
    Remote(String),
    Env(String),
}

#[derive(Debug, Clone)]
pub enum Op {
    App(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseType {
    Text,
    Natural,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Builtin {
    Bool,
    Natural,
    Integer,
    Double,
    Text,
    List,
    Optional,
    OptionalNone,
    Type,
    Kind,
    Sort,
    NaturalBuild,
    NaturalFold,
    NaturalIsZero,
    NaturalEven,
    NaturalOdd,
    NaturalToInteger,
    NaturalShow,
    NaturalSubtract,
    IntegerToDouble,
    IntegerShow,
    IntegerNegate,
    IntegerClamp,
    DoubleShow,
    ListBuild,
    ListFold,
    ListLength,
    ListHead,
    ListLast,
    ListIndexed,
    ListReverse,
    TextShow,
    TextReplace,
}

pub trait Visitor<T> {
    fn visit_expr(&mut self, expr: &Expr) -> T;
}
