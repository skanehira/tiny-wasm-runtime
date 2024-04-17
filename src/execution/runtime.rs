use super::{
    store::{FuncInst, InternalFuncInst, Store},
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
                    self.stack.push(value.clone());
                }
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
                    match func {
                        FuncInst::Internal(func) => self.push_frame(&func.clone()),
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
}
