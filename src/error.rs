use thiserror::Error;

use crate::bytecode::Value;


#[derive(Debug)]
pub enum Error {
    RuntimeError(RuntimeError),
    CompileError(CompileError),
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Runtime encountered error during execution: {0}")]
    Basic(String),
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Frame underflow")]
    FrameUnderflow,
    #[error("Cannot call value {0:?} as a function.")]
    FunctionCall(Value),
    #[error("Internal error (this is probably a bug): {0}")]
    InternalBug(String)
}

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("Invalid redifinition of variable in the same scope: {0}.")]
    VarRedefinition(String, usize),     // varname, span
    #[error("Trying to access undefined variable: {0}.")]
    VarUndefined(String, usize),     // varname, span
    #[error("Internal error (this is probably a bug): {0}")]
    InternalBug(String),
    #[error("Internal error (this is probably a bug): {0}")]
    Basic(String),
}