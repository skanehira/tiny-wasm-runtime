#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryArg {
    pub align: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub block_type: BlockType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockType {
    Empty,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Expr {
    Value(ExprValue),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ExprValue {
    I32(i32),
}

impl From<i32> for Expr {
    fn from(value: i32) -> Self {
        Self::Value(ExprValue::I32(value))
    }
}

impl From<&Expr> for usize {
    fn from(val: &Expr) -> Self {
        match val {
            Expr::Value(value) => match value {
                ExprValue::I32(i) => *i as usize,
            },
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FuncType {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct FunctionLocal {
    pub type_count: u32,
    pub value_type: ValueType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportKind {
    Func(u32),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ImportKind {
    Func(u32),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Limits {
    pub min: u32,
    pub max: Option<u32>,
}
