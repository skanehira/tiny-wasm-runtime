#[derive(Debug, PartialEq, Clone)]
pub struct MemoryArg {
    pub align: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    End,
    Call(u32),
    LocalGet(u32),
    LocalSet(u32),
    I32Store(MemoryArg),
    I32Const(i32),
}
