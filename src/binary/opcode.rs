use num_derive::FromPrimitive;

#[derive(Debug, FromPrimitive, PartialEq)]
pub enum Opcode {
    If = 0x04,
    End = 0x0B,
    Return = 0x0F,
    LocalGet = 0x20,
    LocalSet = 0x21,
    I32Store = 0x36,
    I32Const = 0x41,
    I32LtS = 0x48,
    I32Add = 0x6A,
    I32Sub = 0x6B,
    Call = 0x10,
}
