use std::collections::HashMap;

use crate::binary::{
    instruction::Instruction,
    module::Module,
    types::{ExportDesc, FuncType, ImportDesc, ValueType},
};
use anyhow::{anyhow, bail, Result};

pub const PAGE_SIZE: u32 = 65536; // 64Ki

#[derive(Clone)]
pub struct Func {
    pub locals: Vec<ValueType>,
    pub body: Vec<Instruction>,
}

#[derive(Clone)]
pub struct InternalFuncInst {
    pub func_type: FuncType,
    pub code: Func,
}

#[derive(Debug, Clone)]
pub struct ExternalFuncInst {
    pub module: String,
    pub func: String,
    pub func_type: FuncType,
}

#[derive(Clone)]
pub enum FuncInst {
    Internal(InternalFuncInst),
    External(ExternalFuncInst),
}

pub struct ExportInst {
    pub name: String,
    pub desc: ExportDesc,
}

#[derive(Default)]
pub struct ModuleInst {
    pub exports: HashMap<String, ExportInst>,
}

#[derive(Default, Debug, Clone)]
pub struct MemoryInst {
    pub data: Vec<u8>,
    pub max: Option<u32>,
}

#[derive(Default)]
pub struct Store {
    pub funcs: Vec<FuncInst>,
    pub module: ModuleInst,
    pub memories: Vec<MemoryInst>,
}

impl Store {
    pub fn new(module: Module) -> Result<Self> {
        let func_type_idxs = match module.function_section {
            Some(ref idexs) => idexs.clone(),
            _ => vec![],
        };

        let mut funcs = vec![];
        let mut memories = vec![];

        if let Some(ref import_section) = module.import_section {
            for import in import_section {
                let module_name = import.module.clone();
                let field = import.field.clone();
                let func_type = match import.desc {
                    ImportDesc::Func(type_idx) => {
                        let Some(ref func_types) = module.type_section else {
                            bail!("not found type_section")
                        };

                        let Some(func_type) = func_types.get(type_idx as usize) else {
                            bail!("not found func type in type_section")
                        };

                        func_type.clone()
                    }
                };

                let func = FuncInst::External(ExternalFuncInst {
                    module: module_name,
                    func: field,
                    func_type,
                });
                funcs.push(func);
            }
        }

        if let Some(ref code_section) = module.code_section {
            for (func_body, type_idx) in code_section.iter().zip(func_type_idxs.into_iter()) {
                let Some(ref func_types) = module.type_section else {
                    bail!("not found type_section")
                };

                let Some(func_type) = func_types.get(type_idx as usize) else {
                    bail!("not found func type in type_section")
                };

                let mut locals = Vec::with_capacity(func_body.locals.len());
                for local in func_body.locals.iter() {
                    for _ in 0..local.type_count {
                        locals.push(local.value_type.clone());
                    }
                }

                let func = FuncInst::Internal(InternalFuncInst {
                    func_type: func_type.clone(),
                    code: Func {
                        locals,
                        body: func_body.code.clone(),
                    },
                });
                funcs.push(func);
            }
        }

        let mut exports = HashMap::default();
        if let Some(ref sections) = module.export_section {
            for export in sections {
                let name = export.name.clone();
                let export_inst = ExportInst {
                    name: name.clone(),
                    desc: export.desc.clone(),
                };
                exports.insert(name, export_inst);
            }
        };
        let module_inst = ModuleInst { exports };

        if let Some(ref sections) = module.memory_section {
            for memory in sections {
                let min = memory.limits.min * PAGE_SIZE;
                let memory = MemoryInst {
                    data: vec![0; min as usize],
                    max: memory.limits.max,
                };
                memories.push(memory);
            }
        }

        if let Some(ref sections) = module.data_section {
            for data in sections {
                let memory = memories
                    .get_mut(data.memory_index as usize)
                    .ok_or(anyhow!("not found memory"))?;

                let offset = data.offset as usize;
                let init = &data.init;

                if offset + init.len() > memory.data.len() {
                    bail!("data is too large to fit in memory");
                }
                memory.data[offset..offset + init.len()].copy_from_slice(init);
            }
        }

        Ok(Self {
            funcs,
            memories,
            module: module_inst,
        })
    }
}

#[cfg(test)]
mod test {
    use super::Store;
    use crate::binary::module::Module;
    use anyhow::Result;

    #[test]
    fn init_memory() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/memory.wat")?;
        let module = Module::new(&wasm)?;
        let store = Store::new(module)?;
        assert_eq!(store.memories.len(), 1);
        assert_eq!(store.memories[0].data.len(), 65536);
        assert_eq!(&store.memories[0].data[0..5], b"hello");
        assert_eq!(&store.memories[0].data[5..10], b"world");
        Ok(())
    }
}
