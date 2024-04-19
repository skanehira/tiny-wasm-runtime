use super::{
    import::Import,
    store::{ExternalFuncInst, FuncInst, InternalFuncInst, Store},
    value::Value,
};
use crate::binary::{
    instruction::Instruction,
    module::Module,
    types::{ExportDesc, ValueType},
};
use anyhow::{anyhow, bail, Result};

#[derive(Default)]
pub struct Frame {
    pub pc: isize,
    pub sp: usize,
    pub insts: Vec<Instruction>,
    pub arity: usize,
    pub locals: Vec<Value>,
}

#[derive(Default)]
pub struct Runtime {
    pub store: Store,
    pub stack: Vec<Value>,
    pub call_stack: Vec<Frame>,
    pub import: Import,
}

impl Runtime {
    pub fn instantiate(wasm: impl AsRef<[u8]>) -> Result<Self> {
        let module = Module::new(wasm.as_ref())?;
        let store = Store::new(module)?;
        Ok(Self {
            store,
            ..Default::default()
        })
    }

    pub fn add_import(
        &mut self,
        module_name: impl Into<String>,
        func_name: impl Into<String>,
        func: impl FnMut(&mut Store, Vec<Value>) -> Result<Option<Value>> + 'static,
    ) -> Result<()> {
        let import = self.import.entry(module_name.into()).or_default();
        import.insert(func_name.into(), Box::new(func));
        Ok(())
    }

    pub fn call(&mut self, name: impl Into<String>, args: Vec<Value>) -> Result<Option<Value>> {
        let idx = match self
            .store
            .module
            .exports
            .get(&name.into())
            .ok_or(anyhow!("not found export function"))?
            .desc
        {
            ExportDesc::Func(idx) => idx as usize,
        };
        let Some(func_inst) = self.store.funcs.get(idx) else {
            bail!("not found func")
        };
        for arg in args {
            self.stack.push(arg);
        }
        match func_inst {
            FuncInst::Internal(func) => self.invoke_internal(func.clone()),
            FuncInst::External(func) => self.invoke_external(func.clone()),
        }
    }

    fn push_frame(&mut self, func: &InternalFuncInst) {
        let bottom = self.stack.len() - func.func_type.params.len();
        let mut locals = self.stack.split_off(bottom);

        for local in func.code.locals.iter() {
            match local {
                ValueType::I32 => locals.push(Value::I32(0)),
                ValueType::I64 => locals.push(Value::I64(0)),
            }
        }

        let arity = func.func_type.results.len();

        let frame = Frame {
            pc: -1,
            sp: self.stack.len(),
            insts: func.code.body.clone(),
            arity,
            locals,
        };

        self.call_stack.push(frame);
    }

    fn invoke_internal(&mut self, func: InternalFuncInst) -> Result<Option<Value>> {
        let arity = func.func_type.results.len();

        self.push_frame(&func);

        if let Err(e) = self.execute() {
            self.cleanup();
            bail!("failed to execute instructions: {}", e)
        };

        if arity > 0 {
            let Some(value) = self.stack.pop() else {
                bail!("not found return value")
            };
            return Ok(Some(value));
        }
        Ok(None)
    }

    fn invoke_external(&mut self, func: ExternalFuncInst) -> Result<Option<Value>> {
        let args = self
            .stack
            .split_off(self.stack.len() - func.func_type.params.len());
        let module = self
            .import
            .get_mut(&func.module)
            .ok_or(anyhow!("not found module"))?;
        let import_func = module
            .get_mut(&func.func)
            .ok_or(anyhow!("not found function"))?;
        import_func(&mut self.store, args)
    }

    fn execute(&mut self) -> Result<()> {
        loop {
            let Some(frame) = self.call_stack.last_mut() else {
                break;
            };

            frame.pc += 1;

            let Some(inst) = frame.insts.get(frame.pc as usize) else {
                break;
            };

            match inst {
                Instruction::End => {
                    let Some(frame) = self.call_stack.pop() else {
                        bail!("not found frame");
                    };
                    let Frame { sp, arity, .. } = frame;
                    stack_unwind(&mut self.stack, sp, arity)?;
                }
                Instruction::LocalGet(idx) => {
                    let Some(value) = frame.locals.get(*idx as usize) else {
                        bail!("not found local");
                    };
                    self.stack.push(*value);
                }
                Instruction::LocalSet(idx) => {
                    let Some(value) = self.stack.pop() else {
                        bail!("not found value in the stack");
                    };
                    let idx = *idx as usize;
                    frame.locals[idx] = value;
                }
                Instruction::I32Const(value) => self.stack.push(Value::I32(*value)),
                Instruction::I32Add => {
                    let (Some(right), Some(left)) = (self.stack.pop(), self.stack.pop()) else {
                        bail!("not found any value in the stack");
                    };
                    let result = left + right;
                    self.stack.push(result);
                }
                Instruction::Call(idx) => {
                    let Some(func) = self.store.funcs.get(*idx as usize) else {
                        bail!("not found func");
                    };
                    let func_inst = func.clone();
                    match func_inst {
                        FuncInst::Internal(func) => self.push_frame(&func),
                        FuncInst::External(func) => {
                            if let Some(value) = self.invoke_external(func)? {
                                self.stack.push(value);
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn cleanup(&mut self) {
        self.stack = vec![];
        self.call_stack = vec![];
    }
}

pub fn stack_unwind(stack: &mut Vec<Value>, sp: usize, arity: usize) -> Result<()> {
    if arity > 0 {
        let Some(value) = stack.pop() else {
            bail!("not found return value");
        };
        stack.drain(sp..);
        stack.push(value);
    } else {
        stack.drain(sp..);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Runtime;
    use crate::execution::value::Value;
    use anyhow::Result;

    #[test]
    fn execute_i32_add() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_add.wat")?;
        let mut runtime = Runtime::instantiate(wasm)?;
        let tests = vec![(2, 3, 5), (10, 5, 15), (1, 1, 2)];

        for (left, right, want) in tests {
            let args = vec![Value::I32(left), Value::I32(right)];
            let result = runtime.call("add", args)?;
            assert_eq!(result, Some(Value::I32(want)));
        }
        Ok(())
    }

    #[test]
    fn not_found_export_function() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_add.wat")?;
        let mut runtime = Runtime::instantiate(wasm).unwrap();
        let result = runtime.call("fooooo", vec![]);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn func_call() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_call.wat")?;
        let mut runtime = Runtime::instantiate(wasm)?;
        let tests = vec![(2, 4), (10, 20), (1, 2)];

        for (arg, want) in tests {
            let args = vec![Value::I32(arg)];
            let result = runtime.call("call_doubler", args)?;
            assert_eq!(result, Some(Value::I32(want)));
        }
        Ok(())
    }

    #[test]
    fn call_imported_func() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/import.wat")?;
        let mut runtime = Runtime::instantiate(wasm)?;
        runtime.add_import("env", "add", |_, args| {
            let arg = args[0];
            Ok(Some(arg + arg))
        })?;
        let tests = vec![(2, 4), (10, 20), (1, 2)];

        for (arg, want) in tests {
            let args = vec![Value::I32(arg)];
            let result = runtime.call("call_add", args)?;
            assert_eq!(result, Some(Value::I32(want)));
        }
        Ok(())
    }

    #[test]
    fn not_found_imported_func() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/import.wat")?;
        let mut runtime = Runtime::instantiate(wasm)?;
        runtime.add_import("env", "fooooo", |_, _| Ok(None))?;
        let result = runtime.call("call_add", vec![Value::I32(1)]);
        assert!(result.is_err());
        Ok(())
    }
    
    #[test]
    fn i32_const() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/i32_const.wat")?;
        let mut runtime = Runtime::instantiate(wasm)?;
        let result = runtime.call("i32_const", vec![])?;
        assert_eq!(result, Some(Value::I32(42)));
        Ok(())
    }

    #[test]
    fn local_set() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/local_set.wat")?;
        let mut runtime = Runtime::instantiate(wasm)?;
        let result = runtime.call("local_set", vec![])?;
        assert_eq!(result, Some(Value::I32(42)));
        Ok(())
    }
}
