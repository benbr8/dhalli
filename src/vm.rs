
use std::cell::RefCell;
use std::rc::Rc;

use crate::bytecode::{Op, Value, Function, Closure, UpvalueLoc, Upvalue, UpvalI};
use crate::error::RuntimeError;

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


impl Vm {
    pub fn new() -> Self {
        Vm::default()
    }

    pub fn run(&mut self, function: Function) -> Result<Value, RuntimeError> {
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
            // Value::Function(func) => {
            //     self.push_frame(func.clone(), self.stack.len()-nargs-1);  // clone is bad... lets hope for GC
            // },
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
            Op::Concat => {
                let r = self.pop_stack()?;
                let l = self.pop_stack()?;
                if let (Value::String(ls), Value::String(rs)) = (l, r) {
                    self.push_stack(Value::String(ls + &rs));
                } else {
                    Err(RuntimeError::Basic(
                        format!("Concatenation is only allowed for Strings.")
                    ))?
                }
            }


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
                self.call(nargs)?;
            },
            Op::CloseUpvalue(idx) => {
                // println!("Lifting upvalue {idx}");
                let stack_idx = self.frame()?.stack_offset + idx;
                if let Some(j) = self.upvalues.iter()
                    .position(|x| *x.borrow() == UpvalI::Open(stack_idx))
                {
                    let target = self.upvalues.remove(j);
                    target.replace(UpvalI::Closed(self.stack[stack_idx].clone()));
                }
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
            _ => todo!("{:?}", op)
        }
        if self.debug {
            self.print_stack();
        }
        Ok(())
    }

    // fn get_upval(&self, idx: usize, frame_idx: usize) -> Result<usize, RuntimeError> {
    //     match &self.frames[frame_idx].closure.upvalues[idx] {
    //         Upvalue::Open(stack_idx)
    //         UpvalueLoc::Local(idx) => {
    //             Ok(self.frames[frame_idx].stack_offset + idx)
    //         },
    //         UpvalueLoc::Upval(idx) => {
    //             if frame_idx != 0 {
    //                 self.get_upval(*idx, frame_idx - 1)
    //             } else {
    //                 Err(RuntimeError::InternalBug(format!("Upvalue points out of call stack")))
    //             }
    //         }
    //     }
    // }

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
}