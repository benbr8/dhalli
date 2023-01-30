
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
                let (cidx, idx) = self.resolve_variable(&var.0)?;
                self.emit(Op::GetVar(cidx, idx), 0)
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
        let local = Local { name, depth: c.scope_depth };
        if c.locals.contains(&local) {
            Err(Error::CompileError(CompileError::VarRedefinition(local.name, 0)))
        } else {
            println!("Declaring variable {local:?} at index={}, cdepth={}. Locals={:?}", c.locals.len(), compiler_depth, c.locals);
            c.locals.push(local);
            Ok(())
        }
    }

    // this must only be called in the current compiling
    fn resolve_variable(&mut self, name: &str) -> Result<(usize, usize), Error> {
        let mut cidx = self.compilers.len()-1;
        loop {
            if let Some(idx) = self.resolve_local_at_level(name, cidx) {
                println!("Resolving variable {name} to index={idx}, cdepth={cidx}");
                return Ok((cidx, idx));
            } else {
                let is_lambda = self.compilers.get(cidx).unwrap().is_lambda;
                let ok_to_look_up = if cidx == 0 || is_lambda {
                    false
                } else if cidx >= 1 {
                    // cant refer to outer variable if outer frame is a lambda, since
                    // this can only be the argument and it may no longer be in scope
                    if self.compilers.get(cidx-1).unwrap().is_lambda {
                        false
                    } else { true }
                } else { true };
                if !ok_to_look_up {
                    break;
                };
                cidx -= 1;
            }
        }

        // if not in locals, try upvalues
        for cidx in (0..cidx).rev() {
            if let Some(stack_offset) = self.resolve_local_at_level(name, cidx) {
                self.add_upvalue(Upvalue::Local(stack_offset), );
            }
        }


        todo!()
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



        todo!()
    }

    fn add_upvalue(&mut self, upvalue: Upvalue, cidx: usize) {
        self.compilers[cidx].upvalues.push(upvalue);
    }


}
