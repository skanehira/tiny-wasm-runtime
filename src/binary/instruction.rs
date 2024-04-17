#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    End,
    LocalGet(u32),
    I32Add,
    Call(u32),
}
