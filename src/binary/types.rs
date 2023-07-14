use super::instruction::Instruction;

#[derive(Default, Debug, PartialEq)]
pub struct Custom {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub struct Memory {
    pub limits: Limits,
}

#[derive(Debug, PartialEq)]
pub struct Limits {
    pub min: u32,
    pub max: Option<u32>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct FuncType {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    I32, // 0x7F
    I64, // 0x7E
}

impl From<u8> for ValueType {
    fn from(value: u8) -> Self {
        match value {
            0x7F => Self::I32,
            0x7E => Self::I64,
            _ => panic!("invalid value type: {:X}", value),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Import {
    pub module: String,
    pub field: String,
    pub kind: ImportKind,
}

#[derive(Debug, PartialEq)]
pub enum ImportKind {
    Func(u32),
}

#[derive(Debug, PartialEq)]
pub struct Export {
    pub name: String,
    pub kind: ExportKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportKind {
    Func(u32),
}

#[derive(Debug, PartialEq)]
pub struct Data {
    pub memory_idx: u32,
    pub offset: Expr,
    pub init: Vec<u8>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Expr {
    Value(ExprValue),
    //GlobalIndex(usize),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ExprValue {
    I32(i32),
    //I64(i64),
    //F32(f32),
    //F64(f64),
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct FunctionLocal {
    pub type_count: u32,
    pub value_type: ValueType,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Function {
    pub locals: Vec<FunctionLocal>,
    pub code: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub block_type: BlockType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Empty,
}
