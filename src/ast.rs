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

    // let x : t = r in e
    Let(String, Box<Option<Expr>>, Box<Expr>, Box<Expr>),
    // Record
    Record(BTreeMap<String, Expr>),
    // List
    List(Vec<Expr>),

    Var(Var),
    // \(x : A) -> b
    Lambda(String, Box<Expr>, Box<Expr>),  // arg-name, arg-type, expr

    // Operations
    Op(Op),

    // x : t
    Annot(Box<Expr>, Box<Expr>),
    // assert : x
    Assert(Box<Expr>),
    Dbg,
}

#[derive(Debug, Clone)]
pub enum Op {
    App(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum BaseType {
    Text,
    Natural,
}

pub trait Visitor<T> {
    fn visit_expr(&mut self, expr: &Expr) -> T;
}
