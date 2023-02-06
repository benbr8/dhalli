
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use crate::bytecode::{Op, Value, Function, Closure, UpvalueLoc, Upvalue, UpvalI, Builtin};
use crate::error::RuntimeError;

thread_local! {
    static IMPORT_VALS: RefCell<Vec<Value>> = RefCell::new(Vec::new());
    static IMPORT_INDICES: RefCell<HashMap<String, usize>> = RefCell::new(HashMap::new());
}

pub fn add_import_value(name: String, val: Value) -> usize {
    let idx = IMPORT_VALS.with(|imports| {
        imports.borrow_mut().push(val);
        imports.borrow().len() - 1
    });
    IMPORT_INDICES.with(|map| {
        map.borrow_mut().insert(name, idx);
    });
    idx
}

pub fn get_import_index(name: &str) -> Option<usize> {
    IMPORT_INDICES.with(|map| {
        map.borrow().get(name).cloned()
    })
}

fn get_import_value(import_idx: usize) -> Result<Value, RuntimeError> {
    IMPORT_VALS.with(|imports| {
        imports.borrow()
            .get(import_idx).cloned()
            .ok_or_else(|| RuntimeError::Basic("Failed to import function".to_string()))
    })
}

#[derive(Default)]
pub struct Vm {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    pub debug: bool,
    upvalues: Vec<Upvalue>,   // stack_idx, frame_id, upval_id
}

struct CallFrame {
    closure: Closure,
    ip: usize,
    stack_offset: usize,
}

impl CallFrame {

    fn advance(&mut self) -> Op {
        let r = self.closure.func.chunk.code[self.ip].clone(); // TODO: this is bad
        self.ip += 1;
        r
    }

    fn peek(&self) -> Op {
        self.closure.func.chunk.code[self.ip].clone()
    }
}


pub fn run_function(function: Function, debug: bool) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new();
    vm.debug = debug;
    vm.run(function)
}


impl Vm {
    fn new() -> Self {
        Vm::default()
    }

    fn run(&mut self, function: Function) -> Result<Value, RuntimeError> {
        if self.debug {
            println!("============ FUNCTION ============");
            println!("{:?}", function);
            println!("\n");
        }
        let closure = Closure::new(function);
        self.stack.push(Value::Closure(closure));  // this is pretty bad.. we shouldn't need to keep two function copies around.
        self.call(0)?;
        while !self.done() {
            self.step()?;
        }
        // result is on top of stack
        self.pop_stack()
    }

    fn done(&self) -> bool {
        self.frames.len() == 0
    }

    fn push_frame(&mut self, closure: Closure, stack_offset: usize) {
        let frame = CallFrame { closure, ip: 0, stack_offset };
        self.frames.push(frame);
    }

    fn drop_frame(&mut self) -> Result<(), RuntimeError> {
        self.stack.truncate(self.frame()?.stack_offset);
        if self.frames.pop().is_some() { Ok(()) }
        else { Err(RuntimeError::FrameUnderflow) }
    }

    fn call(&mut self, nargs: usize) -> Result<(), RuntimeError> {
        let func_val = self.peek_stack(nargs)?;
        match func_val {
            Value::Closure(closure) => {
                if closure.func.arity as usize != nargs {
                    Err(RuntimeError::InternalBug("Wrong number of arguments passed into closure.".to_string()))?
                }
                self.push_frame(closure.clone(), self.stack.len()-nargs-1);
            },
            _ => Err(RuntimeError::FunctionCall(func_val.clone()))?,
        }
        Ok(())
    }

    fn step(&mut self) -> Result<(), RuntimeError> {
        let op = self.frame_mut().advance();

        if self.debug {
            println!("Executing: {:?}", op);
            println!("Bytecode: {:?}", self.frame()?.closure.func.chunk.code);
        }
        match op {

            Op::Add => {
                let r = self.pop_stack()?;
                let l = self.pop_stack()?;
                match (l, r) {
                    (Value::Natural(l), Value::Natural(r)) =>
                        self.push_stack(Value::Natural(l + r)),
                    _ => todo!()
                }
            },
            Op::TextAppend => {
                let r = self.pop_stack()?;
                let l = self.pop_stack()?;
                if let (Value::String(ls), Value::String(rs)) = (l, r) {
                    self.push_stack(Value::String(ls + &rs));
                } else {
                    Err(RuntimeError::Basic(
                        format!("Concatenation is only allowed for Strings.")
                    ))?
                }
            },
            Op::ListAppend => {
                let r = self.pop_stack()?;
                let l = self.pop_stack()?;
                match (l, r) {
                    (Value::List(mut li), Value::List(mut ri)) => {
                        li.append(&mut ri);
                        self.push_stack(Value::List(li));
                    },
                    _ => Err(RuntimeError::Basic(format!("ListAppend expects two lists.")))?
                }
            },
            Op::Equal => {
                let r = self.pop_stack()?;
                let l = self.pop_stack()?;
                match (l, r) {
                    (Value::Bool(li), Value::Bool(ri)) => {
                        self.push_stack(Value::Bool(li == ri));
                    },
                    _ => Err(RuntimeError::Basic(format!("Equal can only be used on bools.")))?
                }
            },
            Op::NotEqual => {
                let r = self.pop_stack()?;
                let l = self.pop_stack()?;
                match (l, r) {
                    (Value::Bool(li), Value::Bool(ri)) => {
                        self.push_stack(Value::Bool(li != ri));
                    },
                    _ => Err(RuntimeError::Basic(format!("NotEqual can only be used on bools.")))?
                }
            },
            Op::And => {
                let r = self.pop_stack()?;
                let l = self.pop_stack()?;
                match (l, r) {
                    (Value::Bool(li), Value::Bool(ri)) => {
                        self.push_stack(Value::Bool(li && ri));
                    },
                    _ => Err(RuntimeError::Basic(format!("And can only be used on bools.")))?
                }
            },
            Op::Or => {
                let r = self.pop_stack()?;
                let l = self.pop_stack()?;
                match (l, r) {
                    (Value::Bool(li), Value::Bool(ri)) => {
                        self.push_stack(Value::Bool(li || ri));
                    },
                    _ => Err(RuntimeError::Basic(format!("Or can only be used on bools.")))?
                }
            },
            Op::Combine => {
                let mut r = self.pop_stack()?;
                let mut l = self.pop_stack()?;
                combine_record(&mut l, &mut r)?;
                self.push_stack(l);
            },
            Op::Prefer => {
                let r = self.pop_stack()?;
                let mut l = self.pop_stack()?;
                match (&mut l, r) {
                    (Value::Record(li), Value::Record(ri)) => {
                        for (k, v) in ri {
                            li.insert(k.clone(), v.clone());
                        }
                        self.push_stack(l);
                    },
                    _ => Err(RuntimeError::Basic(format!("Prefer expression can only be used on records.")))?
                }
            },
            Op::CreateRecord(n) => {
                let mut map = BTreeMap::new();
                for _ in 0..n {
                    let val = self.pop_stack()?;
                    let name = self.pop_stack()?;
                    if let Value::String(s) = name {
                        map.insert(s, val);
                    }
                }
                self.push_stack(Value::Record(map));
            },
            Op::CreateList(n) => {
                let mut list = Vec::new();
                for _ in 0..n {
                    list.push(self.pop_stack()?);
                }
                self.push_stack(Value::List(list));
            },
            Op::Constant(const_idx) => self.stack.push(self.func().chunk.get_constant(const_idx)?),
            Op::Closure(const_idx) => {
                let func = self.func().chunk.get_constant(const_idx)?;
                let mut closure = if let Value::Function(func) = func {
                    Closure::new(func)
                } else { Err(RuntimeError::InternalBug(format!("Closure requires a function.")))? };

                // let frame = self.frame_mut();
                while let Op::Upval(upval) = self.frame()?.peek() {
                    let upval = match upval {
                        UpvalueLoc::Local(idx) => {
                            let stack_idx = self.frame()?.stack_offset + idx;
                            if let Some(existing) = self.upvalues.iter().rev()
                                .find(|x| *x.borrow() == UpvalI::Open(stack_idx))
                            {
                                existing.clone()
                            } else {
                                Rc::new(RefCell::new(UpvalI::Open(stack_idx)))
                            }
                        },
                        UpvalueLoc::Upval(idx) => {
                            self.frame()?.closure.upvalues[idx].clone()
                        },
                    };
                    println!("Pushing upvalue {upval:?} created closure");
                    closure.upvalues.push(upval.clone());
                    self.upvalues.push(upval);
                    self.frame_mut().advance();
                }
                self.push_stack(Value::Closure(closure));
            },
            Op::Call(nargs) => {
                if let Value::Builtin(b) = self.peek_stack(nargs)?.clone() {
                    self.apply_builtin_fn(&b)?;
                } else {
                    self.call(nargs)?;
                }
            },
            Op::CloseUpvalue(idx) => {
                // println!("Lifting upvalue {idx}");
                let stack_idx = self.frame()?.stack_offset + idx;
                let val = self.stack[stack_idx].clone();
                self.close_upvalue(stack_idx, val);
            },
            Op::CloseUpvalueBeneath => {
                let r = self.pop_stack()?;
                let val = self.pop_stack()?;
                let stack_idx = self.stack.len();
                self.close_upvalue(stack_idx, val);
                self.push_stack(r);

            },
            Op::Pop => { self.pop_stack()?; },
            Op::PopBeneath => {
                self.stack.remove(self.stack.len()-2);
            },
            Op::Return => {
                let ret = self.pop_stack()?;
                self.drop_frame()?;
                self.push_stack(ret);
            },
            Op::GetVar(idx) => {
                // println!("{} {}", self.frame().stack_offset, idx);
                let stack_idx = self.frame()?.stack_offset + idx;
                let val = self.stack[stack_idx].clone();  // this is also bad, we might be copying a lot of function code around
                self.push_stack(val);
            },
            Op::GetUpval(idx) => {
                let val = match &*self.frame()?.closure.upvalues[idx].borrow() {
                    UpvalI::Open(stack_idx) => self.stack[*stack_idx].clone(),
                    UpvalI::Closed(val) => val.clone(),
                };
                self.push_stack(val);
            },
            Op::Import(import_idx) => {
                self.push_stack(get_import_value(import_idx)?);
            },
            Op::Builtin(b) => {
                self.push_stack(Value::Builtin(b));
            },
            _ => todo!("{:?}", op)
        }
        if self.debug {
            self.print_stack();
        }
        Ok(())
    }

    fn apply_builtin_fn(&mut self, b: &Builtin) -> Result<(), RuntimeError> {
        match b {

            Builtin::Some => {
                let val = self.pop_stack()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                self.push_stack(Value::Option(Some(Box::new(val))));
            },
            Builtin::NaturalOdd => {
                let val = self.pop_stack()?.assume_natural()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                self.push_stack(Value::Bool(val % 2 == 1));
            },
            Builtin::NaturalEven => {
                let val = self.pop_stack()?.assume_natural()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                self.push_stack(Value::Bool(val % 2 == 0));
            },
            Builtin::NaturalIsZero => {
                let val = self.pop_stack()?.assume_natural()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                self.push_stack(Value::Bool(val == 0));
            },
            Builtin::NaturalToInteger => {
                let val = self.pop_stack()?.assume_natural()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                self.push_stack(Value::Integer(val as i64));
            },
            Builtin::NaturalSubtract => {
                let from = self.pop_stack()?.assume_natural()?;
                let this = self.pop_stack()?.assume_natural()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                let r = if this >= from { 0 } else { from - this };
                self.push_stack(Value::Natural(r));
            },
            Builtin::NaturalShow => {
                let val = self.pop_stack()?.assume_natural()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                self.push_stack(Value::String(format!("{val}")));
            },
            // Builtin::NaturalFold => ,
            // Builtin::NaturalBuild => ,
            Builtin::IntegerNegate => {
                let val = self.pop_stack()?.assume_integer()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                self.push_stack(Value::Integer(-val));
            },
            Builtin::IntegerClamp => {
                let val = self.pop_stack()?.assume_integer()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                let r = if val < 0 { 0 } else { val as u64 };
                self.push_stack(Value::Natural(r));
            },
            Builtin::IntegerShow => {
                let val = self.pop_stack()?.assume_integer()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                self.push_stack(Value::String(format!("{val}")));
            },
            // Builtin::IntegerToDouble => ,
            Builtin::TextReplace => {
                let text = self.pop_stack()?.assume_string()?;
                let replacement = self.pop_stack()?.assume_string()?;
                let pattern = self.pop_stack()?.assume_string()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                self.push_stack(Value::String(text.replace(&pattern, &replacement)));
            },
            // Builtin::TextShow => ,
            Builtin::ListLength => {
                let val = self.pop_stack()?.assume_list()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                self.push_stack(Value::Natural(val.len() as u64));
            },
            Builtin::ListReverse => {
                let mut val = self.pop_stack()?.assume_list()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                val.reverse();
                self.push_stack(Value::List(val));
            },
            Builtin::ListHead => {
                let val = self.pop_stack()?.assume_list()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                let r = if !val.is_empty() {
                    Some(Box::new(val[0].clone()))
                } else { None };
                self.push_stack(Value::Option(r));
            },
            Builtin::ListLast => {
                let mut val = self.pop_stack()?.assume_list()?;
                let _ = self.pop_stack()?;  // remove Value::Builtin
                let r = if !val.is_empty() {
                    Some(Box::new(val.pop().unwrap()))
                } else { None };
                self.push_stack(Value::Option(r));
            },
            _ => todo!("{b:?}")
        }
        Ok(())
    }

    fn print_stack(&self) {
        let frame_starts: Vec<usize> = self.frames.iter().map(|frame| {
            frame.stack_offset
        }).collect();
        println!("========= STACK =========");
        for (idx, val) in self.stack.iter().enumerate() {
            if let Some(frame_idx) = frame_starts.iter().position(|a| a == &idx) {
                print!("{frame_idx:>3} >");
            } else { print!("     ") }
            print!("{idx:04}    ");
            match val {
                Value::Closure(c) => {
                    println!("Closure:");
                    println!("             Code:  {:?}", c.func.chunk.code);
                    println!("             Const: {:?}", c.func.chunk.constants);
                    println!("             Upval: {:?}", c.upvalues);
                },
                _ => println!("{val:?}")
            }
            // println!("{:04}    {:?}", idx, val);
        }
        println!("");
    }

    fn frame(&self) -> Result<&CallFrame, RuntimeError> {
        self.frames.last().ok_or_else(|| RuntimeError::InternalBug(format!("Call stack is empty")))
    }
    fn enclosing(&self) -> Result<&CallFrame, RuntimeError> {
        let len = self.frames.len();
        if len >= 2 {
            Ok(&self.frames[len-2])
        } else {
            Err(RuntimeError::InternalBug(format!("Outermost frame does not have enclosing frame.")))
        }
    }
    fn frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn func(&self) -> &Function {
        &self.frames.last().unwrap().closure.func
    }

    fn pop_stack(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or_else(|| RuntimeError::StackUnderflow)
    }
    fn push_stack(&mut self, val: Value) {
        self.stack.push(val);
    }
    fn peek_stack(&self, n: usize) -> Result<&Value, RuntimeError> {
        if self.stack.len() >= n+1 {
            Ok(&self.stack.get(self.stack.len()-n-1).unwrap())
        } else {
            Err(RuntimeError::StackUnderflow)
        }
    }
    fn close_upvalue(&mut self, stack_idx: usize, val: Value) {
        if let Some(j) = self.upvalues.iter()
            .position(|x| *x.borrow() == UpvalI::Open(stack_idx))
        {
            let target = self.upvalues.remove(j);
            target.replace(UpvalI::Closed(val));
        }
    }
}


fn combine_record(l: &mut Value, r: &mut Value) -> Result<(), RuntimeError> {
    if let (Value::Record(li), Value::Record(ri)) = (l, r) {
        for (name, val) in ri {
            if let Some(left) = li.get_mut(name) {
                combine_record(left, val)?;
            } else {
                li.insert(name.clone(), val.clone());  // TODO: there must be a way to transfer ownership instead?
            }
        }
        Ok(())
    } else {
        Err(RuntimeError::Basic("Combine expression can only be usedd with records.".to_string()))
    }
}