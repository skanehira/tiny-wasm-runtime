use super::types::Block;

#[derive(Debug, PartialEq, Clone)]
pub struct MemoryArg {
    pub align: u32,
    pub offset: u32,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    If(Block),
    End,
    Return,
    Call(u32),
    LocalGet(u32),
    LocalSet(u32),
    I32Store(MemoryArg),
    I32Const(i32),
    I32LtS,
    I32Add,
    I32Sub,
}
