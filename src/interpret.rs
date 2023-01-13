
use std::collections::BTreeMap;

use anyhow::{Result, anyhow};

use crate::{ast::{Expr, Visitor, BaseType, Var, Op}, env::Environment};

pub fn interpret(expr: &Expr) -> Result<Expr> {
    let mut interpreter = Interpreter::new();
    interpreter.interpret(expr)
}

struct Interpreter {
    env: Environment
}

impl Interpreter {
    fn new() -> Self {
        Self { env: Environment::new() }
    }
    fn interpret(&mut self, expr: &Expr) -> Result<Expr> {
        self.visit_expr(expr)
    }


    fn builtin(&self, name: &str) -> Option<Expr> {
        // return builtin expr if pattern is matched
        match name {
            "Text" => Some(Expr::BaseType(BaseType::Text)),
            "Natural" => Some(Expr::BaseType(BaseType::Natural)),
            _ => None,
        }
    }
}

impl Visitor<Result<Expr>> for Interpreter {
    fn visit_expr(&mut self, expr: &Expr) -> Result<Expr> {
        match expr {
            Expr::Let(name, t, r, e) => {
                println!("Visiting Let");
                let val = self.visit_expr(r)?;

                if let Some(ref t) = **t {
                    let t = self.visit_expr(t)?;
                    check_type(&val, &t)?;
                }

                self.env.env.define(name.clone(), val.clone())?;
                self.env.push();
                let result = self.visit_expr(e)?;
                self.env.pop();
                Ok(result)
            },
            Expr::Record(map) => {
                let mut result = BTreeMap::new();
                for (name, e) in map.iter() {
                    result.insert(name.clone(), self.visit_expr(e)?);
                }
                Ok(Expr::Record(result))
            },
            Expr::List(vec) => {
                let mut list = Vec::new();
                for item in vec {
                    list.push(self.visit_expr(item)?);
                }
                Ok(Expr::List(list))
            },
            Expr::Text(vec) => {
                let mut result = "".to_string();
                for (s, e) in vec {
                    result = result + s;
                    if let Some(e) = e {
                        let r = self.visit_expr(e)?;
                        match r {
                            Expr::TextLit(s) => result = result + &s,
                            _ => Err(anyhow!("Text can only be interpolated by Text espression."))?,
                        }
                    }
                }
                Ok(Expr::TextLit(result))
            },
            Expr::Var(var) => {
                println!("Visiting Var");
                if let Some(e) = self.builtin(&var.0) {
                    if var.1 != 0 { Err(anyhow!("Can't use index on builtin identifier {}", var.0)) }
                    else { Ok(e) }
                } else {
                    // TODO: implement x@n
                    let e = self.env.env.get(&var.0)?.clone();
                    self.visit_expr(&e)
                }
            },
            Expr::Op(op) => {
                match op {
                    Op::App(l, r) => {
                        let l = self.visit_expr(l)?;
                        let r = self.visit_expr(r)?;
                        
                        match l {
                            Expr::Lambda(arg_name, arg_type, body) => {
                                let r = self.visit_expr(&r)?;
                                let t = self.visit_expr(&arg_type)?;
                                check_type(&r, &t)?;
                                self.env.push();
                                self.env.env.define(arg_name.clone(), r)?;
                                let result = self.visit_expr(&body);
                                self.env.pop();
                                result
                            },
                            _ => todo!("{l:?}")
                        }
                    },
                    _ => todo!()
                }
            },
            Expr::Annot(e, t) => {
                let t = self.visit_expr(t)?;
                let r = self.visit_expr(e)?;
                check_type(&r, &t)?;
                Ok(r)
            },


            // These evaluate to themselves
            Expr::TextLit(_)
            | Expr::Num(_)
            | Expr::Lambda(_, _, _)
            | Expr::BaseType(_) => {
                Ok(expr.clone())
            },
            _ => todo!("{expr:?}")
        }
    }
}

// check type of resolved literals and types
fn check_type(lit: &Expr, t: &Expr) -> Result<()> {
    let matches = match (lit, t) {
        (Expr::Text(_), Expr::BaseType(t)) => matches!(t, &BaseType::Text),
        (Expr::TextLit(_), Expr::BaseType(t)) => matches!(t, &BaseType::Text),
        _ => false
    };

    if matches {
        Ok(())
    } else {
        Err(anyhow!("Expression {lit:?} did not match type {t:?}."))
    }
}