use std::collections::BTreeMap;
use crate::{naive_double::NaiveDouble, bytecode::Builtin};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Var ( pub String, pub usize );  // label, index

pub struct Node {
    expr: Expr,
    span: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {

    // Some
    Some(Box<Expr>),

    Text(Vec<(String, Option<Expr>)>),
    TextLit(String),
    BoolLit(bool),
    NaturalLit(u64),
    IntegerLit(i64),
    DoubleLit(NaiveDouble),
    RecordLit(Vec<(String, Expr)>),
    Builtin(Builtin),
    // let x : t = r in e
    LetIn(Vec<(String, Option<Expr>, Expr)>, Box<Expr>),
    Let(String, Box<Option<Expr>>, Box<Expr>, Box<Expr>),
    // Record
    RecordType(BTreeMap<String, Expr>),
    Record(BTreeMap<String, Expr>),
    // List
    ListLit(Vec<Expr>),
    ListType(Box<Expr>),
    // Union
    UnionType(BTreeMap<String, Option<Expr>>),
    // UnionItem
    // union type, name of variant, Literal
    UnionItem(BTreeMap<String, Option<Expr>>, String, Option<Box<Expr>>),

    Var(Var),
    // \(x : A) -> b
    Select(Box<Expr>, String),
    Lambda(String, Box<Expr>, Box<Expr>),  // arg-name, arg-type, expr
    FnType(Box<Expr>, Box<Expr>),
    Application(Vec<Expr>),

    // Operations
    Op(Op),
    Plus(Box<Expr>, Box<Expr>),
    Combine(Box<Expr>, Box<Expr>),
    TextAppend(Box<Expr>, Box<Expr>),
    ListAppend(Box<Expr>, Box<Expr>),
    Equal(Box<Expr>, Box<Expr>),


    // x : t
    IfThenElse(Box<Expr>, Box<Expr>, Box<Expr>),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    App(Vec<Expr>),
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



pub trait Visitor<T> {
    fn visit_expr(&mut self, expr: &Expr) -> T;
}
