
use std::{rc::Rc, cell::RefCell, collections::BTreeMap};

use crate::error::RuntimeError;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub code: Vec<Op>,
    pub constants: Vec<Value>,
    pub spans: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Import(usize),
    Builtin(Builtin),
    Pop,
    PopBeneath,
    Call(usize), // arg_cnt
    Return,
    Closure(usize),
    Upval(UpvalueLoc), // separate from Closure to not inflate Op too much
    CloseUpvalueBeneath,
    CloseUpvalue(usize),
    Constant(usize),
    GetVar(usize),
    GetUpval(usize),
    CreateRecord(usize),
    CreateList(usize),
    Add,
    TextAppend,
    ListAppend,
    Equal,
    NotEqual,
    And,
    Or,
    Combine,
    Prefer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Natural(u64),
    String(String),
    Bool(bool),
    Function(Function),
    Closure(Closure),
    Record(BTreeMap<String, Value>),
    List(Vec<Value>),
}


pub type Upvalue = Rc<RefCell<UpvalI>>;



#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpvalI {
    Open(usize),
    Closed(Value),
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpvalueLoc {
    Local(usize),
    Upval(usize),
}


#[derive(Clone, PartialEq, Eq)]
pub struct Function {
    pub arity: u8,
    pub chunk: Chunk,
}


impl std::fmt::Debug for Function {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Function")
        //  .field("x", &self.x)
        //  .field("y", &self.y)
            .finish()
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Closure {
    pub func: Function,
    pub upvalues: Vec<Upvalue>,
}

impl Closure {
    pub fn new(func: Function) -> Self {
        Self { func, upvalues: Vec::new() }
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

    pub fn get_constant(&self, idx: usize) -> Result<Value, RuntimeError> {
        if let Some(val) = self.constants.get(idx) {
            Ok(val.clone())
        } else {
            Err(RuntimeError::Basic(format!("Could not access constant. Index out of range: {}", idx)))
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


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Builtin {
    NaturalFold,
    NaturalBuild,
    NaturalIsZero,
    NaturalEven,
    NaturalOdd,
    NaturalToInteger,
    NaturalShow,
    IntegerToDouble,
    IntegerShow,
    IntegerNegate,
    IntegerClamp,
    NaturalSubtract,
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
    Bool,
    True,
    False,
    Optional,
    None,
    Natural,
    Integer,
    Double,
    Text,
    List,
    Type,
    Kind,
    Sort,
}
