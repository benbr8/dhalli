
use std::{rc::Rc, cell::RefCell, collections::BTreeMap};

use crate::error::{RuntimeError, CompileError};

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
    Builtin(Builtin),
    Natural(u64),
    Integer(i64),
    String(String),
    Bool(bool),
    Option(Option<Box<Value>>),
    Record(BTreeMap<String, Value>),
    List(Vec<Value>),
    Function(Function),
    Closure(Closure),
}

impl Value {
    pub fn assume_string(self) -> Result<String, RuntimeError> {
        if let Value::String(val) = self {
            Ok(val)
        } else { Err(RuntimeError::Basic(format!("Expected String, got {self:?} instead."))) }
    }
    pub fn assume_natural(self) -> Result<u64, RuntimeError> {
        if let Value::Natural(val) = self {
            Ok(val)
        } else { Err(RuntimeError::Basic(format!("Expected Natural, got {self:?} instead."))) }
    }
    pub fn assume_integer(self) -> Result<i64, RuntimeError> {
        if let Value::Integer(val) = self {
            Ok(val)
        } else { Err(RuntimeError::Basic(format!("Expected Integer, got {self:?} instead."))) }
    }
    pub fn assume_builtin(self) -> Result<Builtin, RuntimeError> {
        if let Value::Builtin(val) = self {
            Ok(val)
        } else { Err(RuntimeError::Basic(format!("Expected Builtin, got {self:?} instead."))) }
    }
    pub fn assume_bool(self) -> Result<bool, RuntimeError> {
        if let Value::Bool(val) = self {
            Ok(val)
        } else { Err(RuntimeError::Basic(format!("Expected Bool, got {self:?} instead."))) }
    }
    pub fn assume_list(self) -> Result<Vec<Value>, RuntimeError> {
        if let Value::List(val) = self {
            Ok(val)
        } else { Err(RuntimeError::Basic(format!("Expected List, got {self:?} instead."))) }
    }
    pub fn assume_record(self) -> Result<BTreeMap<String, Value>, RuntimeError> {
        if let Value::Record(val) = self {
            Ok(val)
        } else { Err(RuntimeError::Basic(format!("Expected Record, got {self:?} instead."))) }
    }
    pub fn assume_function(self) -> Result<Function, RuntimeError> {
        if let Value::Function(val) = self {
            Ok(val)
        } else { Err(RuntimeError::Basic(format!("Expected Function, got {self:?} instead."))) }
    }
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
    pub fn push_op_below(&mut self, depth: usize, op: Op, span: usize) {
        let len = self.code.len();
        self.code.insert(len - depth, op);
        self.spans.insert(len - depth, span);
    }

    pub fn peek_op(&self) -> &Op {
        self.code.last().unwrap()
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
    Some, // different from spec
}

pub fn builtin_fn_args(builtin: &Builtin) -> Result<usize, CompileError> {
    match builtin {
        Builtin::NaturalFold => Ok(1),
        Builtin::NaturalBuild => Ok(1),
        Builtin::NaturalIsZero => Ok(1),
        Builtin::NaturalEven => Ok(1),
        Builtin::NaturalOdd => Ok(1),
        Builtin::NaturalToInteger => Ok(1),
        Builtin::NaturalSubtract => Ok(2),
        Builtin::NaturalShow => Ok(1),
        Builtin::IntegerToDouble => Ok(1),
        Builtin::IntegerShow => Ok(1),
        Builtin::IntegerNegate => Ok(1),
        Builtin::IntegerClamp => Ok(1),
        Builtin::DoubleShow => Ok(1),
        Builtin::ListBuild => Ok(1),
        Builtin::ListFold => Ok(1),
        Builtin::ListLength => Ok(1),
        Builtin::ListHead => Ok(1),
        Builtin::ListLast => Ok(1),
        Builtin::ListIndexed => Ok(1),
        Builtin::ListReverse => Ok(1),
        Builtin::TextShow => Ok(1),
        Builtin::TextReplace => Ok(3),
        Builtin::Some => Ok(1),
        _ => Err(CompileError::InternalBug("Only builtin functions may have arguments.".to_string())),
    }
}