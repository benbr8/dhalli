
use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Result, anyhow, Context};

use crate::{ast::{Expr, Visitor, Builtin, Var, Op, Import}, env::Environment, import::{import_file_local, import_env}};

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
            "Integer" => Some(Expr::Builtin(Builtin::Integer)),
            "Double" => Some(Expr::Builtin(Builtin::Double)),
            "Type" => Some(Expr::Builtin(Builtin::Type)),
            "List" => Some(Expr::Builtin(Builtin::List)),
            "Bool" => Some(Expr::Builtin(Builtin::Bool)),
            "True" => Some(Expr::BoolLit(true)),
            "False" => Some(Expr::BoolLit(false)),
            "List/fold" => Some(Expr::Builtin(Builtin::ListFold)),
            "Double/show" => Some(Expr::Builtin(Builtin::DoubleShow)),
            "Natural/even" => Some(Expr::Builtin(Builtin::NaturalEven)),
            "Natural/isZero" => Some(Expr::Builtin(Builtin::NaturalIsZero)),
            "Natural/subtract" => Some(Expr::Builtin(Builtin::NaturalSubtract)),
            "Natural/toInteger" => Some(Expr::Builtin(Builtin::NaturalToInteger)),
            "Integer/clamp" => Some(Expr::Builtin(Builtin::IntegerClamp)),
            "Integer/negate" => Some(Expr::Builtin(Builtin::IntegerNegate)),
            _ => None,
        }
    }

    // select from
    fn select_from(&mut self, n: &str, e: &Expr) -> Result<Expr> {
        match e {
            Expr::Record(map) => {
                if let Some(r) = map.get(n) {
                    // r should already be evaluated
                    Ok(r.clone())
                } else {
                    Err(anyhow!("Cannot find selector {n}"))
                }
            },
            Expr::UnionType(variants) => {  // this will construct a union item
                let union_item = Expr::UnionItem(variants.clone(), n.to_string(), None);
                Ok(union_item)
            },
            _ => todo!("{e:?}")
        }
    }

    fn resolve_var(&mut self, e: &Expr) -> Result<Expr> {
        match e {
            Expr::Var(_) => self.visit_expr(e),
            _ => Ok(e.clone())
        }
    }


    // check type of resolved literals and types
    fn check_type(&mut self, lit: &Expr, t: &Expr) -> Result<()> {
        // println!("Checking type: {:?}  vs  {:?}", lit, t);

        let matches = match (lit, t) {
            (Expr::Text(_), Expr::Builtin(t)) => if matches!(t, &Builtin::Text) { Ok(()) } else { Err(anyhow!("Type mismatch.")) },
            (Expr::TextLit(_), Expr::Builtin(t)) => if matches!(t, &Builtin::Text) { Ok(()) } else { Err(anyhow!("Type mismatch.")) },
            (Expr::BoolLit(_), Expr::Builtin(t)) => if matches!(t, &Builtin::Bool) { Ok(()) } else { Err(anyhow!("Type mismatch.")) },
            (Expr::NaturalLit(_), Expr::Builtin(t)) => if matches!(t, &Builtin::Natural) { Ok(()) } else { Err(anyhow!("Type mismatch.")) },
            (Expr::IntegerLit(_), Expr::Builtin(t)) => if matches!(t, &Builtin::Integer) { Ok(()) } else { Err(anyhow!("Type mismatch.")) },
            (Expr::DoubleLit(_), Expr::Builtin(t)) => if matches!(t, &Builtin::Double) { Ok(()) } else { Err(anyhow!("Type mismatch.")) },

            // Builtin
            (Expr::Builtin(b), Expr::Builtin(t)) => {
                match b {
                    Builtin::Bool
                    | Builtin::Double
                    | Builtin::Natural
                    | Builtin::Integer => if matches!(t, &Builtin::Type) { Ok(()) } else { Err(anyhow!("Type mismatch.")) },
                    Builtin::DoubleShow => Ok(()),
                    _ => Err(anyhow!("Type mismatch.")),
                }
            },

            // Record
            (Expr::Record(lit), Expr::RecordType(t)) => {
                if lit.len() == t.len() && lit.keys().all(|k| t.contains_key(k)) {
                    for (k, v) in lit {
                        let t = t.get(k).unwrap();
                        self.check_type(v, t).with_context(|| format!("Checking record item {:?} to be of type {:?}", k, t))?;
                    }
                    Ok(())
                } else {
                    let lit_keys = lit.keys().collect::<Vec<&String>>();
                    let type_keys = t.keys().collect::<Vec<&String>>();
                    Err(anyhow!("Record keys did not match type: {:#?} vs {:#?}", lit_keys, type_keys))
                }
            },

            // List
            (Expr::List(lit), Expr::ListType(t)) => {
                for item in lit {
                    self.check_type(item, t).with_context(|| format!("Checking list item to be of type {:?}.", &t))?;
                }
                Ok(())
            }

            // Lambda
            (Expr::Lambda(_, arg_type, _), Expr::FnType(l, _)) => {
                // For now only check argument type until we implement Lit->Type inference
                let a = self.visit_expr(arg_type)?;
                // dont check for now
                // compare_type(&a, l)?;
                Ok(())
            },

            // Builtin functions
            (Expr::Builtin(b), Expr::FnType(_, _)) => {
                match b {
                    // dont handle these for now
                    Builtin::DoubleShow
                    | Builtin::NaturalEven
                    | Builtin::NaturalBuild
                    | Builtin::NaturalFold
                    | Builtin::NaturalOdd
                    | Builtin::NaturalIsZero
                    | Builtin::NaturalShow
                    | Builtin::NaturalSubtract
                    | Builtin::NaturalToInteger
                    | Builtin::IntegerClamp
                    | Builtin::IntegerNegate
                    | Builtin::IntegerShow
                    | Builtin::IntegerToDouble => Ok(()),
                    _ => Err(anyhow!("Type mismatch."))
                }
            }

            (Expr::RecordType(_), Expr::Builtin(t)) => if matches!(t, &Builtin::Type) { Ok(()) } else { Err(anyhow!("Type mismatch.")) },
            _ => Err(anyhow!("Type mismatch."))
        };

        matches.with_context(|| format!("Expression {:?} did not match type {:?}", lit, t))

    }

    fn process_op(&mut self, op: &Op) -> Result<Expr> {
        match op {
            Op::App(vec_rev) => {
                // println!("len= {}", vec_rev.len());
                // println!("vec= {:#?}", vec_rev);
                let mut vec_rev = vec_rev.clone();
                // let l;

                let l = self.visit_expr(&vec_rev.pop().unwrap())?;

                match &l {
                    Expr::Lambda(arg_name, arg_type, body) => {
                        // println!("App::Lambda: l={:#?} vec={:#?}", &l, &vec_rev);
                        let r = self.visit_expr(&vec_rev.pop().unwrap())?;

                        let t = self.visit_expr(&arg_type)?;
                        self.check_type(&r, &t)?;
                        self.env.push();
                        // println!("Defining {:?} = {:?}", &arg_name, &r);
                        self.env.env.define(arg_name.clone(), r)?;
                        // println!("Env= {:#?}", &self.env);
                        let r = self.visit_expr(body)?;
                        let ret = if !vec_rev.is_empty() {
                            vec_rev.push(r);
                            self.visit_expr(&Expr::Op(Op::App(vec_rev)))?
                        } else {
                            r
                        };
                        self.env.pop();
                        Ok(ret)
                    },
                    Expr::Let(_, _, _, _) => {
                        // Unresolved let is probably from an import saved as a variable.
                        // we resolve it and apply it to the rest of the call stack
                        todo!()
                    },
                    Expr::Builtin(b) => {
                        match b {
                            Builtin::List => {
                                let r = self.visit_expr(&vec_rev.pop().unwrap())?;
                                if !vec_rev.is_empty() {
                                    Err(anyhow!("Cannot Apply ListType to anything."))
                                } else {
                                    Ok(Expr::ListType(Box::new(r)))
                                }
                            },
                            Builtin::ListFold => {
                                let list_type = Expr::ListType(Box::new(self.visit_expr(&vec_rev.pop().unwrap())?));
                                let input = self.visit_expr(
                                    &Expr::Annot(Box::new(vec_rev.pop().unwrap()),
                                    Box::new(list_type))
                                )?;
                                let out_type = self.visit_expr(&vec_rev.pop().unwrap())?;
                                let function = vec_rev.pop().unwrap();
                                let mut last = vec_rev.pop().unwrap();
                                if let Expr::List(i) = input {
                                    for j in i.iter().rev() {
                                        last = self.visit_expr(&Expr::Op(Op::App(vec![last, j.clone(), function.clone()])))?;
                                    }
                                }
                                let ret = self.visit_expr(
                                    &Expr::Annot(Box::new(last),
                                    Box::new(out_type))
                                )?;
                                if vec_rev.is_empty() {
                                    Ok(ret)
                                } else {
                                    vec_rev.push(ret);
                                    self.visit_expr(&Expr::Op(Op::App(vec_rev)))
                                }
                            },
                            Builtin::DoubleShow => {
                                let r = self.visit_expr(&vec_rev.pop().unwrap())?;
                                let result = match r {
                                    Expr::DoubleLit(dbl) => Ok(Expr::TextLit(f64::from(dbl).to_string())),
                                    _ => Err(anyhow!("Double/show can only be used with Double literal.")),
                                };

                                if !vec_rev.is_empty() {
                                    Err(anyhow!("Cannot apply Text literal to anything."))
                                } else {
                                    result
                                }
                            },
                            Builtin::NaturalEven => {
                                let r = self.visit_expr(&vec_rev.pop().unwrap())?;
                                let result = match r {
                                    Expr::NaturalLit(n) => Ok(Expr::BoolLit(n % 2 == 0)),
                                    _ => Err(anyhow!("Natural/even can only be used with Natural literal.")),
                                };

                                if !vec_rev.is_empty() {
                                    Err(anyhow!("Cannot apply Bool literal to anything."))
                                } else {
                                    result
                                }
                            },
                            Builtin::NaturalOdd => {
                                let r = self.visit_expr(&vec_rev.pop().unwrap())?;
                                let result = match r {
                                    Expr::NaturalLit(n) => Ok(Expr::BoolLit(n % 2 == 1)),
                                    _ => Err(anyhow!("Natural/odd can only be used with Natural literal.")),
                                };

                                if !vec_rev.is_empty() {
                                    Err(anyhow!("Cannot apply Bool literal to anything."))
                                } else {
                                    result
                                }
                            },
                            Builtin::NaturalIsZero => {
                                let r = self.visit_expr(&vec_rev.pop().unwrap())?;
                                let result = match r {
                                    Expr::NaturalLit(n) => Ok(Expr::BoolLit(n == 0)),
                                    _ => Err(anyhow!("Natural/isZero can only be used with Natural literal.")),
                                };

                                if !vec_rev.is_empty() {
                                    Err(anyhow!("Cannot apply Bool literal to anything."))
                                } else {
                                    result
                                }
                            },
                            Builtin::NaturalSubtract => {
                                let l = self.visit_expr(&vec_rev.pop().unwrap())?;
                                let r = self.visit_expr(&vec_rev.pop().unwrap())?;
                                let result = match (l, r) {
                                    (Expr::NaturalLit(l), Expr::NaturalLit(r)) => Ok(Expr::NaturalLit( if l > r { l - r } else { 0 })),
                                    _ => Err(anyhow!("Natural/isZero can only be used with Natural literal.")),
                                };

                                if !vec_rev.is_empty() {
                                    Err(anyhow!("Cannot apply Natural literal to anything."))
                                } else {
                                    result
                                }
                            },
                            Builtin::NaturalToInteger => {
                                let n = self.visit_expr(&vec_rev.pop().unwrap())?;
                                let result = match n {
                                    Expr::NaturalLit(n) => Ok(Expr::IntegerLit( n as i64 )),
                                    _ => Err(anyhow!("Natural/isZero can only be used with Natural literal.")),
                                };

                                if !vec_rev.is_empty() {
                                    Err(anyhow!("Cannot apply Integer literal to anything."))
                                } else {
                                    result
                                }
                            },
                            Builtin::IntegerClamp => {
                                let r = self.visit_expr(&vec_rev.pop().unwrap())?;
                                let result = match r {
                                    Expr::IntegerLit(i) => Ok(Expr::NaturalLit(std::cmp::max(i, 0) as u64)),
                                    _ => Err(anyhow!("Integer/clamp can only be used with Integer literal.")),
                                };

                                if !vec_rev.is_empty() {
                                    Err(anyhow!("Cannot apply Natural literal to anything."))
                                } else {
                                    result
                                }
                            },
                            Builtin::IntegerNegate => {
                                let r = self.visit_expr(&vec_rev.pop().unwrap())?;
                                let result = match r {
                                    Expr::IntegerLit(i) => Ok(Expr::IntegerLit(-i)),
                                    _ => Err(anyhow!("Integer/clamp can only be used with Integer literal.")),
                                };

                                if !vec_rev.is_empty() {
                                    Err(anyhow!("Cannot apply Integer literal to anything."))
                                } else {
                                    result
                                }
                            },
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
                        // self.visit_expr(l).or(self.visit_expr(r))
                        // disable alternative import for now
                        self.visit_expr(l)
                    },
                    _ => Err(anyhow!("'?' can only be used on Imports. Got expressions: {l:?} '+' {r:?}" )),
                }
            },
            Op::Equivalent(l, r) => {
                let l = self.visit_expr(l)?;
                let r = self.visit_expr(r)?;

                // TODO: is it really this easy?
                //Ok(Expr::BoolLit(l == r))
                // it is not: example: prelude identity.dhall
                Ok(Expr::BoolLit(true))

            },
            Op::And(l, r) => {
                let l = self.visit_expr(l)?;
                let r = self.visit_expr(r)?;

                match (l, r) {
                    (Expr::BoolLit(li), Expr::BoolLit(ri)) => Ok(Expr::BoolLit(li && ri)),
                    _ => Err(anyhow!("And expression only valid for bool type.")),
                }
            },
            Op::Or(l, r) => {
                let l = self.visit_expr(l)?;
                let r = self.visit_expr(r)?;
                match (l, r) {
                    (Expr::BoolLit(li), Expr::BoolLit(ri)) => Ok(Expr::BoolLit(li || ri)),
                    _ => Err(anyhow!("Or expression only valid for bool type.")),
                }
            },
            _ => todo!("{op:?}")
        }
    }
}

impl Visitor<Result<Expr>> for Interpreter {
    fn visit_expr(&mut self, expr: &Expr) -> Result<Expr> {
        // println!("Processing expr: {:?}", expr);
        match expr {
            Expr::Let(name, t, r, e) => {
                let val = self.visit_expr(r)?;

                if let Some(ref t) = **t {
                    let t = self.visit_expr(t)?;
                    self.check_type(&val, &t)?;
                }

                if let Expr::Lambda(_, _, _) = val {
                    // if target is lambda expression, we need the whole expression to stay intact
                    // because it will need to be called multiple times
                    Ok(expr.clone())
                } else {
                    self.env.env.define(name.clone(), val.clone())?;
                    self.env.push();
                    let result = self.visit_expr(e)?;
                    self.env.pop();
                    Ok(result)
                }

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
                if let Some(e) = self.builtin(&var.0) {
                    if var.1 != 0 { Err(anyhow!("Can't use index on builtin identifier {}", var.0)) }
                    else { Ok(e) }
                } else {
                    // TODO: implement x@n
                    let e = self.env.env.get(&var.0)?.clone();
                    println!("Resolved '{}' to: {:#?}", &var.0, &e);
                    self.visit_expr(&e)
                }
            },
            Expr::Op(op) => {
                self.process_op(op)
            },
            Expr::Select(e, n) => {
                let e = self.visit_expr(e)?;
                self.select_from(n, &e)
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
            },
            Expr::IfThenElse(test, thene, elsee) => {
                let test_result = self.visit_expr(test)?;
                match test_result {
                    Expr::BoolLit(b) => {
                        if b { self.visit_expr(thene) } else { self.visit_expr(elsee) }
                    },
                    _ => Err(anyhow!("IfThenElse test must be of type Bool, got this instead: {:?}", &test_result))
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
                    Import::Env(env_var) => {
                        println!("env:{}", env_var);
                        import_env(env_var.clone(), &self.file)
                    },
                    _ => todo!("{import:?} from {:?}", &self.file)
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
                // this currently makes trouble because l/r are evaluated like variables, but they dont exist
                Ok(Expr::FnType(Box::new(self.resolve_var(l)?), Box::new(self.resolve_var(r)?)))

                // Ok(expr.clone())
            }

            // These evaluate to themselves
            Expr::TextLit(_)
            | Expr::BoolLit(_) | Expr::IntegerLit(_) | Expr::NaturalLit(_) | Expr::DoubleLit(_)
            | Expr::Lambda(_, _, _)
            | Expr::UnionType(_) // this will need to change
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