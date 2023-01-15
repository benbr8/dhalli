
use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Result, anyhow};

use crate::{ast::{Expr, Visitor, Builtin, Var, Op, Import}, env::Environment, import::import_file_local};

pub fn interpret(expr: &Expr, file: &PathBuf) -> Result<Expr> {
    let path = std::fs::canonicalize(file)?;
    let mut interpreter = Interpreter::new();
    interpreter.interpret(expr, path)
}

struct Interpreter {
    env: Environment,
    file: PathBuf,
}

impl Interpreter {
    fn new() -> Self {
        Self { env: Environment::new(), file: PathBuf::new() }
    }
    fn interpret(&mut self, expr: &Expr, file: PathBuf) -> Result<Expr> {
        self.file = file;
        self.visit_expr(expr)
    }


    fn builtin(&self, name: &str) -> Option<Expr> {
        // return builtin expr if pattern is matched
        match name {
            "Text" => Some(Expr::Builtin(Builtin::Text)),
            "Natural" => Some(Expr::Builtin(Builtin::Natural)),
            "Type" => Some(Expr::Builtin(Builtin::Type)),
            "List" => Some(Expr::Builtin(Builtin::List)),
            "True" => Some(Expr::BoolLit(true)),
            "False" => Some(Expr::BoolLit(false)),
            _ => None,
        }
    }

    
    // check type of resolved literals and types
    fn check_type(&mut self, lit: &Expr, t: &Expr) -> Result<()> {
        
        let matches = match (lit, t) {
            (Expr::Text(_), Expr::Builtin(t)) => matches!(t, &Builtin::Text),
            (Expr::TextLit(_), Expr::Builtin(t)) => matches!(t, &Builtin::Text),

            // Record
            (Expr::Record(lit), Expr::RecordType(t)) => {
                if lit.len() == t.len() && lit.keys().all(|k| t.contains_key(k)) {
                    for (k, v) in lit {
                        self.check_type(v, t.get(k).unwrap())?;
                    }
                    true
                } else { false }
            },

            // List
            (Expr::List(lit), Expr::ListType(t)) => {
                for item in lit {
                    self.check_type(item, t)?;
                }
                true
            }

            // Lambda
            (Expr::Lambda(_, arg_type, _), Expr::FnType(l, _)) => {
                // For now only check argument type until we implement Lit->Type inference
                let a = self.visit_expr(arg_type)?;
                compare_type(&a, l)?;
                true
            },

            (Expr::RecordType(_), Expr::Builtin(t)) => matches!(t, &Builtin::Type),
            _ => false
        };

        if matches {
            Ok(())
        } else {
            Err(anyhow!("Expression {lit:?} did not match type {t:?}."))
        }
    }

    fn process_op(&mut self, op: &Op) -> Result<Expr> {
        match op {
            Op::App(l, r) => {
                let l = self.visit_expr(l)?;
                let r = self.visit_expr(r)?;
                
                match l {
                    Expr::Lambda(arg_name, arg_type, body) => {
                        let r = self.visit_expr(&r)?;
                        let t = self.visit_expr(&arg_type)?;
                        self.check_type(&r, &t)?;
                        self.env.push();
                        self.env.env.define(arg_name.clone(), r)?;
                        let result = self.visit_expr(&body);
                        self.env.pop();
                        result
                    },
                    Expr::Builtin(b) => {
                        match b {
                            Builtin::List => Ok(Expr::ListType(Box::new(r))),
                            _ => todo!("{b:?}")
                        }
                    },
                    _ => todo!("{l:?}")
                }
            },
            Op::TextAppend(l, r) => {
                let l = self.visit_expr(l)?;
                let r = self.visit_expr(r)?;
                match (&l, &r) {
                    (Expr::TextLit(ls), Expr::TextLit(rs)) => Ok(Expr::TextLit(format!("{ls}{rs}"))),
                    _ => Err(anyhow!("'++' may only concatenate Text literals. Got {l:?} and {r:?} instead.")),
                }
            },
            Op::ListAppend(l, r) => {
                let l = self.visit_expr(l)?;
                let r = self.visit_expr(r)?;
                match (l, r) {
                    (Expr::List(mut ls), Expr::List(mut rs)) => {
                        // TODO: type check when lit-to-type is implemented
                        ls.append(&mut rs);
                        Ok(Expr::List(ls))
                    },
                    _ => Err(anyhow!("'#' may only concatenate Lists.")),
                }
            },
            Op::Equal(l, r) => {
                let l = self.visit_expr(l)?;
                let r = self.visit_expr(r)?;
                match (&l, &r) {
                    (Expr::BoolLit(lb), Expr::BoolLit(rb)) => {
                        Ok(Expr::BoolLit(lb == rb))
                    },
                    _ => Err(anyhow!("'==' may only compare Bools. Got {l:?} and {r:?} instead." )),
                }
            },
            Op::NotEqual(l, r) => {
                let l = self.visit_expr(l)?;
                let r = self.visit_expr(r)?;
                match (&l, &r) {
                    (Expr::BoolLit(lb), Expr::BoolLit(rb)) => {
                        Ok(Expr::BoolLit(lb != rb))
                    },
                    _ => Err(anyhow!("'!=' may only compare Bools. Got {l:?} and {r:?} instead." )),
                }
            },
            Op::Plus(l, r) => {
                let l = self.visit_expr(l)?;
                let r = self.visit_expr(r)?;
                match (&l, &r) {
                    (Expr::IntegerLit(li), Expr::IntegerLit(ri)) => {
                        Ok(Expr::IntegerLit(li + ri))
                    },
                    (Expr::NaturalLit(li), Expr::NaturalLit(ri)) => {
                        Ok(Expr::NaturalLit(li + ri))
                    },
                    // (Expr::DoubleLit(li), Expr::DoubleLit(ri)) => {
                    //     Ok(Expr::DoubleLit(li + ri))
                    // },
                    _ => Err(anyhow!("Cannot add. Incompatible types: {l:?} '+' {r:?}" )),
                }
            },
            Op::Combine(l, r) => {
                let mut l = self.visit_expr(l)?;
                let mut r = self.visit_expr(r)?;
                
                combine_record(&mut l, &mut r)?;
                Ok(l)
            },
            Op::Prefer(l, r) => {
                let mut l = self.visit_expr(l)?;
                let mut r = self.visit_expr(r)?;

                match (&mut l, &mut r) {
                    (Expr::Record(li), Expr::Record(ri)) => {
                        for (k, v) in ri {
                            li.insert(k.clone(), v.clone());
                        }
                        Ok(l)
                    },
                    _ => Err(anyhow!("'//' can only be used on Records. Got these instead: {l:?} '+' {r:?}" )),
                }
            },
            Op::CombineTypes(l, r) => {
                let mut l = self.visit_expr(l)?;
                let mut r = self.visit_expr(r)?;
                
                combine_record_type(&mut l, &mut r)?;
                Ok(l)
            },
            Op::ImportAlt(l, r) => {
                match (&**l, &**r) {
                    (Expr::Import(_), Expr::Import(_)) => {
                        self.visit_expr(l).or(self.visit_expr(r))
                    },
                    _ => Err(anyhow!("'?' can only be used on Imports. Got expressions: {l:?} '+' {r:?}" )),
                }
            },
            Op::Equivalent(l, r) => {
                let l = self.visit_expr(l)?;
                let r = self.visit_expr(r)?;

                // TODO: is it really this easy?
                Ok(Expr::BoolLit(l == r))
            },
            _ => todo!()
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
                    self.check_type(&val, &t)?;
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
                self.process_op(op)
            },
            Expr::Select(e, n) => {
                let e = self.visit_expr(e)?;
                select_from(n, &e)
            },
            Expr::Annot(e, t) => {
                let t = self.visit_expr(t)?;
                let r = self.visit_expr(e)?;
                self.check_type(&r, &t)?;
                Ok(r)
            },
            Expr::Assert(e) => {
                let result = self.visit_expr(e)?;
                if let Expr::BoolLit(b) = result {
                    if b { Ok(result) }
                    else { Err(anyhow!("Assertion failed: {e:?}"))}
                } else {
                    Err(anyhow!("Assertion failed: {e:?}"))
                }
            }


            Expr::Import(import) => {
                match import {
                    Import::Local(f) => {
                        let path = if f.starts_with('.') {
                            let dir = self.file.parent().unwrap();
                            dir.join(f).to_path_buf()
                        } else {
                            PathBuf::from(f)
                        };
                        println!("{:?}", path);
                        import_file_local(&path)
                    },
                    _ => todo!()
                }
            },

            Expr::RecordType(t) => {
                let mut map = BTreeMap::new();
                for (k, v) in t {
                    map.insert(k.clone(), self.visit_expr(v)?);
                }
                Ok(Expr::RecordType(map))
            },
            Expr::ListType(t) => {
                Ok(Expr::ListType(Box::new(self.visit_expr(t)?)))
            },
            Expr::FnType(l, r) => {
                Ok(Expr::FnType(Box::new(self.visit_expr(l)?), Box::new(self.visit_expr(r)?)))
            }

            // These evaluate to themselves
            Expr::TextLit(_)
            | Expr::BoolLit(_) | Expr::IntegerLit(_) | Expr::NaturalLit(_) //| Expr::DoubleLit(_)
            | Expr::Lambda(_, _, _)
            | Expr::Builtin(_) => {
                Ok(expr.clone())
            },
            _ => todo!("{expr:?}")
        }
    }
}


fn compare_type(t1: &Expr, t2: &Expr) -> Result<()> {
    let matches = match (t1, t2) {
        (Expr::Builtin(t1), Expr::Builtin(t2)) => t1 == t2,
        (Expr::RecordType(t1), Expr::RecordType(t2)) => {
            for (k, v) in t1 {
                compare_type(v, t2.get(k).unwrap())?;
            }
            true
        },
        _ => false,
    };
    if matches {
        Ok(())
    } else {
        Err(anyhow!("Type mismatch: {t1:?} did not match {t2:?}."))
    }
}

fn select_from(n: &str, e: &Expr) -> Result<Expr> {
    match e {
        Expr::Record(map) => {
            if let Some(r) = map.get(n) {
                Ok(r.clone())
            } else {
                Err(anyhow!("Cannot find selector {n}"))
            }
        },
        _ => todo!("{e:?}")
    }
}

fn combine_record(l: &mut Expr, r: &mut Expr) -> Result<()> {
    if let (Expr::Record(li), Expr::Record(ri)) = (l, r) {
        for (k, v) in ri {
            if let Some(left) = li.get_mut(k) {
                combine_record(left, v)?;
            } else {
                li.insert(k.clone(), v.clone());  // TODO: there must be a way to transfer ownership instead?
            }
        }
        Ok(())
    } else {
        Err(anyhow!("Can only combine Record expressions."))
    }
}

// this needs to be a different function than pure records. Think about it.
fn combine_record_type(l: &mut Expr, r: &mut Expr) -> Result<()> {
    if let (Expr::RecordType(li), Expr::RecordType(ri)) = (l, r) {
        for (k, v) in ri {
            if let Some(left) = li.get_mut(k) {
                combine_record_type(left, v)?;
            } else {
                li.insert(k.clone(), v.clone());  // TODO: there must be a way to transfer ownership instead?
            }
        }
        Ok(())
    } else {
        Err(anyhow!("Can only combine Record expressions."))
    }
}

fn check_equivalence(l: &Expr, r: &Expr) -> bool {
    match (l, r) {
        (Expr::Record(li), Expr::Record(ri)) => {
            if li.len() != ri.len() {
                return false;
            }
            for (k, v) in ri {
                if let Some(left) = li.get(k) {
                    if !check_equivalence(left, v) {
                        return false;
                    }
                } else { return false; }
            }
            return true;
        },
        (Expr::List(li), Expr::List(ri)) => {
            if li.len() != ri.len() {
                return false;
            }
            let mut li = li.iter(); let mut ri = ri.iter();
            while let Some(right) = ri.next() {
                if !check_equivalence(li.next().unwrap(), right) {
                    return false;
                }
            }
            return true;
        },
        _ => todo!()
    };
    true
}