

use crate::bytecode::{Op, Value, Function, Closure};
use crate::error::{Error, RuntimeError};

#[derive(Default)]
pub struct Vm {
    stack: Vec<Value>,
    frames: Vec<CallFrame>,
    pub debug: bool,
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
}


impl Vm {
    pub fn new() -> Self {
        Vm::default()
    }

    pub fn run(&mut self, function: Function) -> Result<Value, Error> {
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

    fn drop_frame(&mut self) -> Result<(), Error> {
        self.stack.truncate(self.frame().stack_offset);
        if self.frames.pop().is_some() { Ok(()) }
        else { Err(Error::RuntimeError(RuntimeError::FrameUnderflow)) }
    }

    fn call(&mut self, nargs: usize) -> Result<(), Error> {
        let func_val = self.peek_stack(nargs)?;
        match func_val {
            // Value::Function(func) => {
            //     self.push_frame(func.clone(), self.stack.len()-nargs-1);  // clone is bad... lets hope for GC
            // },
            Value::Closure(closure) => {
                if closure.func.arity as usize != nargs {
                    Err(Error::RuntimeError(RuntimeError::InternalBug("Wrong number of arguments passed into closure.".to_string())))?
                }
                self.push_frame(closure.clone(), self.stack.len()-nargs-1);
            },
            _ => Err(Error::RuntimeError(RuntimeError::FunctionCall(func_val.clone())))?,
        }
        Ok(())
    }

    fn step(&mut self) -> Result<(), Error> {
        let op = self.frame_mut().advance();

        if self.debug {
            println!("Op: {:?}", op);
        }
        match op {
            Op::Constant(const_idx) => self.stack.push(self.func().chunk.get_constant(const_idx)?),
            Op::Closure(const_idx) => {
                let func = self.func().chunk.get_constant(const_idx)?;
                let closure = if let Value::Function(func) = func {
                    Closure::new(func)
                } else { Err(Error::RuntimeError(RuntimeError::InternalBug("Closure requires a function.".to_string())))? };
                self.push_stack(Value::Closure(closure));
            },
            Op::Call(nargs) => {
                self.call(nargs)?;
            },
            Op::Return => {
                let ret = self.pop_stack()?;
                self.drop_frame()?;
                self.push_stack(ret);
            },
            Op::Add => {
                let r = self.pop_stack()?;
                let l = self.pop_stack()?;
                match (l, r) {
                    (Value::Natural(l), Value::Natural(r)) =>
                        self.push_stack(Value::Natural(l + r)),

                    _ => todo!()
                }
            },
            Op::GetVar(idx) => {
                // println!("{} {}", self.frame().stack_offset, idx);
                let stack_idx = self.frame().stack_offset + idx;
                let val = self.stack[stack_idx].clone();  // this is also bad, we might be copying a lot of function code around
                self.push_stack(val);
            },
            _ => todo!("{:?}", op)
        }
        if self.debug {
            self.print_stack();
        }
        Ok(())
    }

    fn print_stack(&self) {
        let frame_starts: Vec<usize> = self.frames.iter().map(|frame| {
            frame.stack_offset
        }).collect();
        println!("========= STACK TRACE =========");
        for (idx, val) in self.stack.iter().enumerate() {
            if let Some(frame_idx) = frame_starts.iter().position(|a| a == &idx) {
                print!("{frame_idx:>3} >");
            } else { print!("     ") }
            println!("{:04}    {:?}", idx, val);
        }
        println!("");
    }

    fn frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }
    fn frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }

    fn func(&self) -> &Function {
        &self.frames.last().unwrap().closure.func
    }

    fn pop_stack(&mut self) -> Result<Value, Error> {
        self.stack.pop().ok_or_else(|| Error::RuntimeError(RuntimeError::StackUnderflow))
    }
    fn push_stack(&mut self, val: Value) {
        self.stack.push(val);
    }
    fn peek_stack(&self, n: usize) -> Result<&Value, Error> {
        if self.stack.len() >= n+1 {
            Ok(&self.stack.get(self.stack.len()-n-1).unwrap())
        } else {
            Err(Error::RuntimeError(RuntimeError::StackUnderflow))
        }
    }
}