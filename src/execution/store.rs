use std::collections::HashMap;

use crate::binary::{
    instruction::Instruction,
    module::Module,
    types::{ExportDesc, FuncType, ImportDesc, ValueType},
};
use anyhow::{bail, Result};

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

#[derive(Default)]
pub struct Store {
    pub funcs: Vec<FuncInst>,
    pub module: ModuleInst,
}

impl Store {
    pub fn new(module: Module) -> Result<Self> {
        let func_type_idxs = match module.function_section {
            Some(ref idexs) => idexs.clone(),
            _ => vec![],
        };

        let mut funcs = vec![];

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

        Ok(Self {
            funcs,
            module: module_inst,
        })
    }
}
