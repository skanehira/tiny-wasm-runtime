#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    End,
    LocalGet(u32),
    LocalSet(u32),
    I32Store { align: u32, offset: u32 },
    I32Const(i32),
    I32Add,
    Call(u32),
}
