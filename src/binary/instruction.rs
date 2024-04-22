use super::types::Block;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    If(Block),
    End,
    Return,
    LocalGet(u32),
    LocalSet(u32),
    I32Store { align: u32, offset: u32 },
    I32Const(i32),
    I32Lts,
    I32Add,
    I32Sub,
    Call(u32),
}
