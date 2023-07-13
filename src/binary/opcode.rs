use num_derive::FromPrimitive;

#[derive(Debug, FromPrimitive, PartialEq)]
#[repr(u8)]
pub enum Opcode {
    End = 0x0B,
    Call = 0x10,
    LocalGet = 0x20,
    LocalSet = 0x21,
    I32Store = 0x36,
    I32Const = 0x41,
}
