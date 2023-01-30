
use crate::error::{Error, RuntimeError};

#[derive(Debug, Default, Clone)]
pub struct Chunk {
    pub code: Vec<Op>,
    pub constants: Vec<Value>,
    pub spans: Vec<usize>,
}

#[derive(Debug, Clone)]
pub enum Op {
    Pop,
    Return,
    Closure(usize),
    Call(usize), // arg_cnt
    Constant(usize),
    GetVar(usize, usize),  // frame_idx, stack_idx
    Add,
}

#[derive(Debug, Clone)]
pub enum Value {
    Natural(u64),
    String(String),
    Function(Function),
    Closure(Closure),
}


#[derive(Debug, Clone)]
pub enum Upvalue {
    Local(usize),
    Upval(usize),
}


#[derive(Debug, Clone)]
pub struct Function {
    pub arity: u8,
    pub chunk: Chunk,
}

#[derive(Debug, Clone)]
pub struct Closure {
    pub func: Function,
}

impl Closure {
    pub fn new(func: Function) -> Self {
        Self { func }
    }
}


impl Chunk {
    pub fn new() -> Chunk {
        Chunk::default()
    }

    pub fn disassemble(&self, name: &str) {
        // todo, add line information
        println!("== {} ==", name);
        for (offset, op) in self.code.iter().enumerate() {
            println!("{:04}    {}", offset, op.to_string());
        }
    }

    pub fn push_op(&mut self, op: Op, span: usize) {
        self.code.push(op);
        self.spans.push(span);
    }

    pub fn get_constant(&self, idx: usize) -> Result<Value, Error> {
        if let Some(val) = self.constants.get(idx) {
            Ok(val.clone())
        } else {
            Err(Error::RuntimeError(RuntimeError::Basic(format!("Could not access constant. Index out of range: {}", idx))))
        }
    }
    pub fn add_constant(&mut self, val: Value) -> usize {
        let idx = self.constants.len();
        self.constants.push(val);
        idx
    }
}



impl Op {
    pub fn to_string(&self) -> String {
        match self {
            Op::Return => "OP_RETURN".to_string(),
            Op::Constant(c_idx) => format!("OP_CONSTANT  {}", c_idx),
            _ => todo!("{:?}", self)
        }
    }
}

impl Function {
    pub fn new() -> Self {
        Self { arity: 0, chunk: Chunk::new() }
    }
}
