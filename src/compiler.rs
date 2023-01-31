
use crate::ast::{Expr, self};
use crate::bytecode::{Op, Value, Function, Upvalue};
use crate::error::{Error, CompileError};


pub fn compile(ast: &Expr) -> Result<Function, Error> {
    let mut compiler = Compiler::new();
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
}

#[derive(Debug)]
struct FunctionCompiler {
    func: Function,
    scope_depth: usize,
    locals: Vec<Local>,
    upvalues: Vec<Upvalue>,
    is_lambda: bool,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Local {
    name: String,
    depth: usize,
    is_captured: bool,
}

impl FunctionCompiler {
    pub fn new(is_lambda: bool) -> Self {
        let locals = vec![Local::default()];
        Self { func: Function::new(), scope_depth: 0, locals, upvalues: Vec::new(), is_lambda }
    }
    // consume compiler and return generated code chunk
    pub fn get_function(self) -> Function {
        self.func
    }
}

impl Compiler {

    pub fn new() -> Self {
        Self { compilers: vec![FunctionCompiler::new(true)] }
    }

    pub fn get_function(mut self) -> Function {
        let base_comp = self.compilers.remove(0);
        base_comp.get_function()
    }


    fn push_compiler(&mut self, is_lambda: bool) {
        self.compilers.push(FunctionCompiler::new(is_lambda));
    }
    fn pop_compiler(&mut self) -> FunctionCompiler {
        self.compilers.pop().unwrap()
    }

    pub fn compile(&mut self, ast: &Expr) -> Result<(), Error> {
        match ast {
            Expr::Op(op) => {
                match op {
                    ast::Op::Plus(l, r) => {
                        self.compile(l)?;
                        self.compile(r)?;
                        self.emit(Op::Add, 0);
                    },
                    _ => todo!(),
                }
            },
            Expr::NaturalLit(val) => {
                let const_idx = self.add_constant(Value::Natural(*val));
                self.emit(Op::Constant(const_idx), 0);
            },
            Expr::LetIn(vec, sub) => {

                self.push_compiler(false);
                self.function().arity = 0;  // let doesn't take arguments
                for (name, _, val) in vec {
                    // computed val is on top of stack
                    self.compile(val)?;
                    // declare val as variable
                    self.declare_variable(name.clone())?;
                }
                self.compile(sub)?;
                self.emit(Op::Return, 0);
                let func = self.pop_compiler().get_function();
                let const_idx = self.add_constant(Value::Function(func));
                self.emit(Op::Closure(const_idx), 0);
                self.emit(Op::Call(0), 0);  // immediately call with 0 args
            },
            Expr::Lambda(arg_name, _, expr) => {
                self.push_compiler(true);
                self.function().arity = 1;  // lambdas always have one argument
                self.declare_variable(arg_name.clone())?;  // Register arg_name to point to first slot of call frame
                self.compile(expr)?;
                self.emit(Op::Return, 0);
                let func = self.pop_compiler().get_function();
                let const_idx = self.add_constant(Value::Function(func));  // add function to constants
                self.emit(Op::Closure(const_idx), 0);  // Refer to constant in bytecode
            },
            Expr::Application(vec) => {
                // parser ensures length of vector is at least 2
                println!("Compiling application: {vec:?}");
                let first = vec.len()-1;
                self.compile(&vec[first])?;
                for j in (0..first).rev() {
                    self.compile(&vec[j])?;
                    self.emit(Op::Call(1), 0);
                    println!("Call(1)");
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
            _ => todo!("ast:?")
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

    fn add_constant(&mut self, val: Value) -> usize {
        self.function().chunk.add_constant(val)
    }

    fn begin_scope(&mut self) {
        self.compiler().scope_depth += 1;
    }
    fn end_scope(&mut self) {
        self.compiler().scope_depth -= 1;
        for j in (self.compiler().locals.len()-1)..0 {
            if self.compiler().locals[j].depth > self.compiler().scope_depth {
                self.emit(Op::Pop, 0);
            } else {
                break;
            }
        }
    }

    // declaring a (local) variable is as simple as mapping the current stack top to a name
    fn declare_variable(&mut self, name: String) -> Result<(), Error> {
        let compiler_depth = self.compilers.len()-1;
        let c = self.compiler();
        let local = Local { name, depth: c.scope_depth, is_captured: false };
        if c.locals.contains(&local) {
            Err(Error::CompileError(CompileError::VarRedefinition(local.name, 0)))
        } else {
            println!("Declaring variable {local:?} at index={}, cdepth={}. Locals={:?}", c.locals.len(), compiler_depth, c.locals);
            c.locals.push(local);
            Ok(())
        }
    }

    // this must only be called in the current compiling
    fn resolve_variable(&mut self, name: &str) -> Result<ResolvedVar, Error> {
        let cidx = self.compilers.len()-1;

        if let Some(idx) = self.resolve_local_at_level(name, cidx) {
            Ok(ResolvedVar::Local(idx))
        } else {
            if let Some(upval_idx) = self.resolve_upvalue_at_level(name, cidx) {
                Ok(ResolvedVar::Upval(upval_idx))
            } else {
                Err(Error::CompileError(CompileError::VarUndefined(name.to_string(), 0)))
            }
        }

    }


    fn resolve_local_at_level(&mut self, name: &str, cidx: usize) -> Option<usize> {
        let compiler = self.compilers.get(cidx).unwrap();
        println!("At cidx={cidx}: locals={:?}", compiler.locals);

        for p in (0..compiler.locals.len()).rev() {
            if &compiler.locals[p].name == name {
                return Some(p);
            }
        }
        return None;
    }

    fn resolve_upvalue_at_level(&mut self, name: &str, cidx: usize) -> Option<usize> {
        if cidx <= 1 {
            return None;
        }
        if let Some(idx) = self.resolve_local_at_level(name, cidx - 1) {
            let up_idx = self.add_upvalue(Upvalue::Local(idx), cidx);
            Some(up_idx)
        } else {
            if let Some(up_idx) = self.resolve_upvalue_at_level(name, cidx - 1) {
                let up_idx = self.add_upvalue(Upvalue::Upval(up_idx), cidx);
                Some(up_idx)
            } else {
                None
            }
        }

    }

    fn add_upvalue(&mut self, upvalue: Upvalue, cidx: usize) -> usize {
        let up_idx = self.compilers[cidx].upvalues.len();
        self.compilers[cidx].upvalues.push(upvalue);
        up_idx
    }


}
