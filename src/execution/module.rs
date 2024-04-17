use std::collections::HashMap;

use crate::{
    binary::types::{FuncType, ValueType},
    Instruction,
};

pub const PAGE_SIZE: u32 = 65536;

#[derive(Debug, Clone, PartialEq)]
pub enum LabelKind {
    If,
    Loop,
    Block,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub kind: LabelKind,
    pub start: Option<isize>,
    pub pc: usize,
    pub sp: usize,
    pub arity: usize,
}

#[derive(Debug, Clone)]
pub struct Func {
    pub type_idx: u32,
    pub locals: Vec<ValueType>,
    pub body: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct InternalFuncInst {
    pub func_type: FuncType,
    pub code: Func,
}

#[derive(Debug, Clone)]
pub struct ExternalFuncInst {
    pub module: String,
    pub field: String, // function name
    pub func_type: FuncType,
}

#[derive(Debug, Clone)]
pub enum FuncInst {
    Internal(InternalFuncInst),
    External(ExternalFuncInst),
}

#[derive(Default, Debug, Clone)]
pub struct MemoryInst {
    pub data: Vec<u8>,
    pub max: Option<u32>,
}


#[derive(Debug, Clone)]
pub enum ExternalValue {
    Func(u32),
    Memory(u32),
}

#[derive(Debug, Clone)]
pub struct ExportInst {
    pub name: String,
    pub desc: ExternalValue,
}

#[derive(Debug, Default, Clone)]
pub struct ModuleInst {
    pub func_types: Vec<FuncType>,
    pub exports: HashMap<String, ExportInst>,
}
