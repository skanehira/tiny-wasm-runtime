use super::{
    instruction::Instruction,
    types::{Expr, FunctionLocal, ImportKind, ExportKind, Limits},
};
use num_derive::FromPrimitive;

#[derive(Debug, PartialEq, Eq, FromPrimitive)]
pub enum SectionCode {
    Custom = 0x00,
    Type = 0x01,
    Import = 0x02,
    Function = 0x03,
    Table = 0x04,
    Memory = 0x05,
    Global = 0x06,
    Export = 0x07,
    Start = 0x08,
    Element = 0x09,
    Code = 0x0a,
    Data = 0x0b,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Memory {
    pub limits: Limits,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Import {
    pub module: String,
    pub field: String,
    pub kind: ImportKind,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Export {
    pub name: String,
    pub kind: ExportKind,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Data {
    pub memory_idx: u32,
    pub offset: Expr,
    pub init: Vec<u8>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub locals: Vec<FunctionLocal>,
    pub code: Vec<Instruction>,
}
