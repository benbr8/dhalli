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

    Text(Vec<(String, Option<Expr>)>),
    TextLit(String),
    BoolLit(bool),
    NaturalLit(u64),
    IntegerLit(i64),
    DoubleLit(f64),
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
    Completion(Box<Expr>, Box<Expr>),
    Equivalent(Box<Expr>, Box<Expr>),
    ImportAlt(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Plus(Box<Expr>, Box<Expr>),
    TextAppend(Box<Expr>, Box<Expr>),
    ListAppend(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Combine(Box<Expr>, Box<Expr>),
    Prefer(Box<Expr>, Box<Expr>),
    CombineTypes(Box<Expr>, Box<Expr>),
    Times(Box<Expr>, Box<Expr>),
    Equal(Box<Expr>, Box<Expr>),
    NotEqual(Box<Expr>, Box<Expr>),
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
