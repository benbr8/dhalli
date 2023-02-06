
use std::path::PathBuf;

use crate::ast::{Expr, self, Var, Import};
use crate::bytecode::{Op, Value, Function, UpvalueLoc, Builtin, builtin_fn_args};
use crate::error::CompileError;
use crate::{import2, vm};


pub fn compile(ast: &Expr, file: PathBuf) -> Result<Function, CompileError> {
    let mut compiler = Compiler::new(file);
    compiler.compile(ast)?;
    let mut function = compiler.get_function();
    function.chunk.push_op(Op::Return, 0);
    Ok(function)
}

enum ResolvedVar {
    Local(usize),
    Upval(usize),
}

#[derive(Debug, Default)]
struct Compiler {
    compilers: Vec<FunctionCompiler>,
    file: PathBuf,
}

#[derive(Debug)]
struct FunctionCompiler {
    func: Function,
    scope_depth: usize,
    locals: Vec<Local>,
    upvalues: Vec<UpvalueLoc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Local {
    name: String,
    depth: usize,
    is_captured: bool,
}

impl FunctionCompiler {
    pub fn new() -> Self {
        let locals = vec![Local::default()];
        Self { func: Function::new(), scope_depth: 0, locals, upvalues: Vec::new() }
    }
    // consume compiler and return generated code chunk
    pub fn get_function(self) -> Function {
        self.func
    }
}

impl Compiler {

    pub fn new(file: PathBuf) -> Self {
        Self { compilers: vec![FunctionCompiler::new()], file }
    }

    pub fn get_function(mut self) -> Function {
        let base_comp = self.compilers.remove(0);
        base_comp.get_function()
    }

    fn push_compiler(&mut self) {
        self.compilers.push(FunctionCompiler::new());
    }
    fn pop_compiler(&mut self) -> FunctionCompiler {
        self.compilers.pop().unwrap()
    }

    pub fn compile(&mut self, ast: &Expr) -> Result<(), CompileError> {
        match ast {
            Expr::Import(import) => {
                match import {
                    Import::Local(file) => {
                        let mut file_dir = self.file.clone();
                        file_dir.pop();
                        let mut path = file_dir.join(file);
                        println!("Joined: {path:?}");
                        path = std::fs::canonicalize(path).unwrap();
                        let path_string = path.to_string_lossy().to_string();
                        let import_idx = if let Some(import_idx) = vm::get_import_index(&path_string) {
                            println!("Getting value from stash (idx={import_idx}): {path_string}.");
                            import_idx
                        } else {
                            // Dhall imports cannot close over values from importing contexts,
                            // and thus can be precompiled and pre-executed and stored as value to
                            // be pushed directly to the stack.
                            let func = import2::import_file_local(&path)?;
                            println!("Got function {:?}.", func.chunk);
                            let val = vm::run_function(func, true).unwrap();
                            let import_idx = vm::add_import_value(path_string, val.clone());
                            println!("Saving value to stash (idx={import_idx}): {val:?}.");
                            import_idx
                        };
                        self.emit(Op::Import(import_idx), 0);
                    },
                    _ => todo!("{import:?}"),
                }
            },
            Expr::NaturalLit(val) => {
                let const_idx = self.add_constant(Value::Natural(*val));
                self.emit(Op::Constant(const_idx), 0);
            },
            Expr::IntegerLit(val) => {
                let const_idx = self.add_constant(Value::Integer(*val));
                self.emit(Op::Constant(const_idx), 0);
            },
            Expr::BoolLit(val) => {
                let const_idx = self.add_constant(Value::Bool(*val));
                self.emit(Op::Constant(const_idx), 0);
            },
            Expr::Text(vec) => {
                let mut n_slices = 0;
                for (s, e) in vec {
                    // println!("{s}, {e:?}");
                    if !s.is_empty() {
                        let const_idx = self.add_constant(Value::String(s.clone()));
                        self.emit(Op::Constant(const_idx), 0);
                    }
                    if let Some(e) = e {
                        self.compile(e)?;
                        n_slices += 1;
                    }
                }
                for _ in 0..n_slices {
                    self.emit(Op::TextAppend, 0)
                }
            },
            Expr::RecordLit(items) => {
                self.begin_scope();
                for (s, e) in items {
                    self.compile(e)?;
                    self.declare_variable(s.clone())?;
                }
                for (s, _) in items {
                    let c = self.add_constant(Value::String(s.clone()));
                    self.emit(Op::Constant(c), 0);
                    self.compile(&Expr::Var(Var(s.clone(), 0)))?;
                }
                self.emit(Op::CreateRecord(items.len()), 0);
                self.end_scope_with_result();
            },
            Expr::ListLit(items) => {
                for e in items {
                    self.compile(e)?;
                }
                self.emit(Op::CreateList(items.len()), 0);
            },
            Expr::LetIn(vec, sub) => {
                self.begin_scope();
                for (name, _, val) in vec {
                    if matches!(val, &Expr::RecordType(_)) {
                        // ignore type definitions
                        continue;
                    }
                    self.compile(val)?;
                    self.declare_variable(name.clone())?;
                }
                self.compile(sub)?;
                self.end_scope_with_result();
            },
            Expr::Lambda(arg_name, _, expr) => {
                self.push_compiler();
                self.function().arity = 1;  // lambdas always have one argument
                self.declare_variable(arg_name.clone())?;  // Register arg_name to point to first slot of call frame
                self.compile(expr)?;
                for (idx, local) in self.compiler().locals.clone().iter().enumerate() {
                    if local.is_captured {
                        self.emit(Op::CloseUpvalue(idx), 0);
                    }
                }
                self.emit(Op::Return, 0);
                let upvalues = self.compiler().upvalues.clone();  // inefficient
                let func = self.pop_compiler().get_function();
                let const_idx = self.add_constant(Value::Function(func));  // add function to constants
                self.emit(Op::Closure(const_idx), 0);  // Refer to constant in bytecode
                for upval in upvalues {
                    self.emit(Op::Upval(upval), 0);
                }
            },
            Expr::Application(vec) => {
                // parser ensures length of vector is at least 2
                println!("Compiling application: {vec:?}");
                // let first = vec.len()-1;
                // self.compile(&vec[first])?;
                // for j in (0..first).rev() {
                //     self.compile(&vec[j])?;
                //     self.emit(Op::Call(1), 0);
                // }

                let len = vec.len();
                let mut j = 1;
                self.compile(&vec[0])?;
                while j < len {
                    if let Op::Builtin(b) = self.peek_op() {
                        // If function to be applied is to be a builtin, compile its arguments then Call(nargs)
                        // Check that nargs does not exceed remaining items in call vector
                        let n_args = builtin_fn_args(b)?;
                        if n_args > len - j {
                            Err(CompileError::Basic(format!("Less arguments then expected for builtin {b:?}: {} instead of {n_args}", j+1)))?
                        }
                        for _ in 0..n_args {
                            self.compile(&vec[j])?;
                            j += 1;
                        }
                        self.emit(Op::Call(n_args), 0);
                    } else {
                        self.compile(&vec[j])?;
                        j += 1;
                        self.emit(Op::Call(1), 0);
                    }
                }
            },
            Expr::Var(var) => {
                // TODO: x@2
                let var = self.resolve_variable(&var.0)?;
                match var {
                    ResolvedVar::Local(idx) => self.emit(Op::GetVar(idx), 0),
                    ResolvedVar::Upval(idx) => self.emit(Op::GetUpval(idx), 0),
                }
            },

            // Operations

            Expr::Plus(l, r) => {
                self.compile(l)?;
                self.compile(r)?;
                self.emit(Op::Add, 0);
            },

            Expr::TextAppend(l, r) => {
                self.compile(l)?;
                self.compile(r)?;
                self.emit(Op::TextAppend, 0);
            },
            Expr::ListAppend(l, r) => {
                self.compile(l)?;
                self.compile(r)?;
                self.emit(Op::ListAppend, 0);
            },
            Expr::Equal(l, r) => {
                self.compile(l)?;
                self.compile(r)?;
                self.emit(Op::Equal, 0);
            },
            Expr::NotEqual(l, r) => {
                self.compile(l)?;
                self.compile(r)?;
                self.emit(Op::NotEqual, 0);
            },
            Expr::And(l, r) => {
                self.compile(l)?;
                self.compile(r)?;
                self.emit(Op::And, 0);
            },
            Expr::Or(l, r) => {
                self.compile(l)?;
                self.compile(r)?;
                self.emit(Op::Or, 0);
            },
            Expr::Combine(l, r) => {
                self.compile(l)?;
                self.compile(r)?;
                self.emit(Op::Combine, 0);
            },
            Expr::Prefer(l, r) => {
                self.compile(l)?;
                self.compile(r)?;
                self.emit(Op::Prefer, 0);
            },


            // Builtin
            Expr::Builtin(b) => {
                match b {
                    Builtin::NaturalSubtract
                    | Builtin::NaturalFold
                    | Builtin::NaturalBuild
                    | Builtin::NaturalIsZero
                    | Builtin::NaturalEven
                    | Builtin::NaturalOdd
                    | Builtin::NaturalToInteger
                    | Builtin::NaturalShow
                    | Builtin::IntegerToDouble
                    | Builtin::IntegerShow
                    | Builtin::IntegerNegate
                    | Builtin::IntegerClamp
                    | Builtin::DoubleShow
                    | Builtin::ListBuild
                    | Builtin::ListFold
                    | Builtin::ListLength
                    | Builtin::ListHead
                    | Builtin::ListLast
                    | Builtin::ListIndexed
                    | Builtin::ListReverse
                    | Builtin::TextShow
                    | Builtin::TextReplace
                        => self.emit(Op::Builtin(b.clone()), 0),
                    _ => todo!("{b:?}"),
                }
            },
            Expr::Some(e) => {
                // wrap some value in Some by using builtin function mechanism
                self.emit(Op::Builtin(Builtin::Some), 0);
                self.compile(e)?;
                self.emit(Op::Call(1), 0);
            },


            // Ignore
            Expr::Annot(e, _) => {
                self.compile(e)?;
            },
            _ => todo!("{ast:?}")
        };
        Ok(())
    }

    fn function(&mut self) -> &mut Function {
        &mut self.compilers.last_mut().unwrap().func
    }

    /// returns current topmost FunctionCompiler
    fn compiler(&mut self) -> &mut FunctionCompiler {
        self.compilers.last_mut().unwrap()
    }

    fn emit(&mut self, op: Op, span: usize) {
        self.function().chunk.push_op(op, span);
    }
    fn emit_below(&mut self, depth: usize, op: Op, span: usize) {
        self.function().chunk.push_op_below(depth, op, span);
    }
    fn peek_op(&mut self) -> &Op {
        self.function().chunk.peek_op()
    }

    fn add_constant(&mut self, val: Value) -> usize {
        self.function().chunk.add_constant(val)
    }

    fn begin_scope(&mut self) {
        self.compiler().scope_depth += 1;
    }
    fn end_scope_with_result(&mut self) {
        self.compiler().scope_depth -= 1;

        // TODO: this could be optimized by selectively closing, and then popping the rest in one go
        for j in (0..self.compiler().locals.len()).rev() {
            if self.compiler().locals[j].depth > self.compiler().scope_depth {
                if self.compiler().locals[j].is_captured {
                    self.emit(Op::CloseUpvalueBeneath, 0);
                } else {
                    self.emit(Op::PopBeneath, 0);
                }
                self.compiler().locals.pop();
            } else {
                break;
            }
        }
    }


    // declaring a (local) variable is as simple as mapping the current stack top to a name
    fn declare_variable(&mut self, name: String) -> Result<(), CompileError> {
        let compiler_depth = self.compilers.len()-1;
        let c = self.compiler();
        let local = Local { name, depth: c.scope_depth, is_captured: false };
        if c.locals.contains(&local) {
            Err(CompileError::VarRedefinition(local.name, 0))
        } else {
            println!("Declaring variable {local:?} at index={}, cdepth={}. Locals={:?}", c.locals.len(), compiler_depth, c.locals);
            c.locals.push(local);
            Ok(())
        }
    }

    // this must only be called in the current compiling
    fn resolve_variable(&mut self, name: &str) -> Result<ResolvedVar, CompileError> {
        let cidx = self.compilers.len()-1;
        println!("Try Resolving {name} at cidx={cidx}");

        if let Some(idx) = self.resolve_local_at_level(name, cidx, false) {
            println!("Resolving {name} locally cidx={cidx}");
            Ok(ResolvedVar::Local(idx))
        } else {
            if let Some(upval_idx) = self.resolve_upvalue_at_level(name, cidx) {
                Ok(ResolvedVar::Upval(upval_idx))
            } else {
                Err(CompileError::VarUndefined(name.to_string(), 0))
            }
        }

    }


    fn resolve_local_at_level(&mut self, name: &str, cidx: usize, capture: bool) -> Option<usize> {
        let compiler = self.compilers.get_mut(cidx).unwrap();

        for p in (0..compiler.locals.len()).rev() {
            if &compiler.locals[p].name == name {
                compiler.locals[p].is_captured = capture;
                return Some(p);
            }
        }
        return None;
    }

    fn resolve_upvalue_at_level(&mut self, name: &str, cidx: usize) -> Option<usize> {
        if cidx <= 1 {
            return None;
        }
        if let Some(stack_offset) = self.resolve_local_at_level(name, cidx-1, true) {
            let up_idx = self.add_upvalue(UpvalueLoc::Local(stack_offset), cidx);
            println!("Resolving {name} locally at cidx={}, upval_idx={up_idx}", cidx-1);
            Some(up_idx)
        } else {
            if let Some(up_idx) = self.resolve_upvalue_at_level(name, cidx - 1) {
                let up_idx = self.add_upvalue(UpvalueLoc::Upval(up_idx), cidx);
                println!("Adding upval at cidx={cidx}, upval_idx={up_idx}");
                Some(up_idx)
            } else {
                None
            }
        }
    }

    fn add_upvalue(&mut self, upvalue: UpvalueLoc, cidx: usize) -> usize {
        println!("Adding upvalue at cidx={cidx}");
        let up_idx = self.compilers[cidx].upvalues.len();
        self.compilers[cidx].upvalues.push(upvalue);
        up_idx
    }


}



