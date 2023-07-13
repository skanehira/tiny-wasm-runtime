#![allow(dead_code, unused_imports)]

use super::{
    instruction::{Instruction, MemoryArg},
    opcode::Opcode,
    section::SectionID,
    types::{
        Block, BlockType, Custom, Data, Export, ExportKind, Expr, ExprValue, FuncType, Function,
        FunctionLocal, Import, ImportKind, Limits, Memory, ValueType,
    },
};
use nom::{
    bytes::complete::{tag, take},
    multi::many0,
    number::complete::{le_u32, le_u8},
    sequence::pair,
    IResult,
};
use nom_leb128::{leb128_i32, leb128_u32};
use num_traits::FromPrimitive as _;

#[derive(Default, Debug, PartialEq)]
pub struct Module {
    pub magic: String,
    pub version: u32,
    pub custom_section: Option<Custom>,
    pub memory_section: Option<Vec<Memory>>,
    pub type_section: Option<Vec<FuncType>>,
    pub function_section: Option<Vec<u32>>,
    pub import_section: Option<Vec<Import>>,
    pub export_section: Option<Vec<Export>>,
    pub data_section: Option<Vec<Data>>,
    pub code_section: Option<Vec<Function>>,
}

impl Module {
    fn new(input: &[u8]) -> IResult<&[u8], Module> {
        let (input, _) = tag(b"\0asm")(input)?;
        let (input, version) = le_u32(input)?;

        let mut module = Module {
            magic: "\0asm".into(),
            version,
            ..Default::default()
        };

        let mut remaining = input;

        // 入力の終端の場合はデコード処理を抜ける
        while !remaining.is_empty() {
            match decode_section_header(remaining) {
                // input: セクションヘッダーを覗いた残りのバイト列
                // (id, size): セクションのIDとサイズ
                Ok((input, (id, size))) => {
                    // セクションのサイズだけバイト列を切り出す
                    // rest は残りのバイト列
                    let (rest, section_bytes) = take(size)(input)?;

                    match id {
                        SectionID::Custom => {
                            // カスタムセクションの読み込み
                            // 残りは存在しないため、タプルの1番目は捨てる
                            let (_, custom) = decode_custom_section(section_bytes)?;
                            module.custom_section = Some(custom);
                        }
                        SectionID::Memory => {
                            let (_, memory) = decode_memory_section(section_bytes)?;
                            module.memory_section = Some(vec![memory]);
                        }
                        SectionID::Type => {
                            let (_, func_types) = decode_type_section(section_bytes)?;
                            module.type_section = Some(func_types);
                        }
                        SectionID::Function => {
                            let (_, func_idx_list) = decode_function_section(section_bytes)?;
                            module.function_section = Some(func_idx_list);
                        }
                        SectionID::Import => {
                            let (_, imports) = decode_import_section(section_bytes)?;
                            module.import_section = Some(imports);
                        }
                        SectionID::Export => {
                            let (_, exports) = decode_export_section(section_bytes)?;
                            module.export_section = Some(exports);
                        }
                        SectionID::Data => {
                            let (_, data) = decode_data_section(section_bytes)?;
                            module.data_section = Some(data);
                        }
                        SectionID::Code => {
                            let (_, funcs) = decode_code_section(section_bytes)?;
                            module.code_section = Some(funcs);
                        }
                        _ => {}
                    };
                    // 残りのバイト列を次のループで読み込む
                    remaining = rest;
                }
                Err(err) => return Err(err),
            }
        }

        Ok((remaining, module))
    }
}

fn decode_section_header(input: &[u8]) -> IResult<&[u8], (SectionID, u32)> {
    let (input, (id, size)) = pair(le_u8, leb128_u32)(input)?;
    Ok((
        input,
        (SectionID::from_u8(id).expect("unexpected section id"), size),
    ))
}

fn decode_name(input: &[u8]) -> IResult<&[u8], String> {
    let (input, size) = leb128_u32(input)?;
    let (input, name) = take(size)(input)?;
    Ok((input, String::from_utf8_lossy(name).into_owned()))
}

fn decode_memory_section(input: &[u8]) -> IResult<&[u8], Memory> {
    let (input, _) = leb128_u32(input)?;
    let (_, limits) = decode_limits(input)?;
    Ok((&[], Memory { limits }))
}

fn decode_import_section(input: &[u8]) -> IResult<&[u8], Vec<Import>> {
    let mut imports = vec![];

    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        // module name
        let (rest, module) = decode_name(input)?;

        // field name
        let (rest, field) = decode_name(rest)?;

        // import kind
        let (rest, import_kind) = le_u8(rest)?;
        let (rest, kind) = match import_kind {
            0x00 => {
                // idxはインポートする関数の型のインデックス
                let (rest, idx) = leb128_u32(rest)?;
                (rest, ImportKind::Func(idx))
            }
            _ => todo!(),
        };

        imports.push(Import {
            module,
            field,
            kind,
        });

        input = rest;
    }

    Ok((&[], imports))
}

fn decode_export_section(input: &[u8]) -> IResult<&[u8], Vec<Export>> {
    let mut exports = vec![];

    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        // name: エクスポートする名前
        let (rest, name) = decode_name(input)?;

        // export kind
        let (rest, export_kind) = le_u8(rest)?;
        let (rest, idx) = leb128_u32(rest)?;
        let kind = match export_kind {
            0x00 => ExportKind::Func(idx),
            _ => todo!(),
        };

        exports.push(Export { name, kind });

        input = rest;
    }

    Ok((&[], exports))
}

fn decode_limits(input: &[u8]) -> IResult<&[u8], Limits> {
    let (mut input, (limits, min)) = pair(leb128_u32, leb128_u32)(input)?;
    let max = if limits == 0 {
        None
    } else {
        let (rest, max) = leb128_u32(input)?;
        input = rest;
        Some(max)
    };

    Ok((input, Limits { min, max }))
}

fn decode_custom_section(input: &[u8]) -> IResult<&[u8], Custom> {
    let (input, name) = decode_name(input)?;
    let data = input.to_vec();
    Ok((&[], Custom { name, data }))
}

fn decode_type_section(input: &[u8]) -> IResult<&[u8], Vec<FuncType>> {
    let mut func_types: Vec<FuncType> = vec![];

    // count: 関数の型定義の数
    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        // NOTE: 本来なら0x60であることをチェックする必要があるが、簡易実装のためスキップ
        let (rest, _) = le_u8(input)?;
        let mut func = FuncType::default();

        // rest: 残りのバイト列
        // size: 引数の数
        let (rest, size) = leb128_u32(rest)?;
        // 引数の数だけバイト列を切り出す
        // types: 引数の型のバイト列
        let (rest, types) = take(size)(rest)?;
        // 引数の型のバイト列を1バイトずつ読み込む
        let (_, types) = many0(le_u8)(types)?;
        func.params = types.into_iter().map(Into::into).collect();

        let (rest, size) = leb128_u32(rest)?;
        let (rest, types) = take(size)(rest)?;
        let (_, types) = many0(le_u8)(types)?;
        func.results = types.into_iter().map(Into::into).collect();

        func_types.push(func);
        input = rest;
    }
    Ok((&[], func_types))
}

fn decode_function_section(input: &[u8]) -> IResult<&[u8], Vec<u32>> {
    let mut func_idx_list = vec![];
    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        let (rest, idx) = leb128_u32(input)?;
        func_idx_list.push(idx);
        input = rest;
    }

    Ok((&[], func_idx_list))
}

fn decode_data_section(input: &[u8]) -> IResult<&[u8], Vec<Data>> {
    let mut data = vec![];
    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        let (rest, memory_idx) = leb128_u32(input)?;
        let (rest, offset) = decode_expr(rest)?;
        let (rest, size) = leb128_u32(rest)?;
        let (rest, init) = take(size)(rest)?;

        data.push(Data {
            memory_idx,
            offset,
            init: init.to_vec(),
        });

        input = rest;
    }
    Ok((&[], data))
}

fn decode_expr(input: &[u8]) -> IResult<&[u8], Expr> {
    let (input, byte) = le_u8(input)?;
    let op = Opcode::from_u8(byte).expect("invalid opcode");
    let (input, value) = match op {
        Opcode::I32Const => {
            let (input, value) = leb128_i32(input)?;
            (input, Expr::Value(ExprValue::I32(value)))
        }
        _ => unreachable!(),
    };
    // NOTE: 本来ならopcodeがend(0x0b)であることをチェックする必要があるが、簡易実装のためスキップ
    let (input, _) = le_u8(input)?;
    Ok((input, value))
}

fn decode_code_section(input: &[u8]) -> IResult<&[u8], Vec<Function>> {
    let mut functions = vec![];
    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        let (rest, size) = leb128_u32(input)?;
        let (rest, body) = take(size)(rest)?;
        let (_, body) = decode_function_body(body)?;
        functions.push(body);
        input = rest;
    }

    Ok((&[], functions))
}

fn decode_function_body(input: &[u8]) -> IResult<&[u8], Function> {
    let mut body = Function::default();

    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        let (rest, type_count) = leb128_u32(input)?;
        let (rest, value_type) = le_u8(rest)?;
        let value_type: ValueType = value_type.into();
        body.locals.push(FunctionLocal {
            type_count,
            value_type,
        });
        input = rest;
    }

    let mut remaining = input;

    while !remaining.is_empty() {
        let (rest, inst) = decode_instructions(remaining)?;
        body.code.push(inst);
        remaining = rest;
    }

    Ok((&[], body))
}

fn decode_block(input: &[u8]) -> IResult<&[u8], Block> {
    let (input, byte) = le_u8(input)?;

    let block_type = if byte == 0x40 {
        BlockType::Empty
    } else {
        let err = nom::error::Error::new(input, nom::error::ErrorKind::Fail);
        return Err(nom::Err::Failure(err));
    };

    Ok((input, Block { block_type }))
}

fn decode_instructions(input: &[u8]) -> IResult<&[u8], Instruction> {
    let (input, byte) = le_u8(input)?;
    let op = Opcode::from_u8(byte).unwrap_or_else(|| panic!("invalid opcode: {:X}", byte));
    let (rest, inst) = match op {
        Opcode::If => {
            let (rest, block) = decode_block(input)?;
            (rest, Instruction::If(block))
        }
        Opcode::End => (input, Instruction::End),
        Opcode::Return => (input, Instruction::Return),
        Opcode::Call => {
            let (rest, idx) = leb128_u32(input)?;
            (rest, Instruction::Call(idx))
        }
        Opcode::LocalGet => {
            let (rest, idx) = leb128_u32(input)?;
            (rest, Instruction::LocalGet(idx))
        }
        Opcode::LocalSet => {
            let (rest, idx) = leb128_u32(input)?;
            (rest, Instruction::LocalSet(idx))
        }
        Opcode::I32Store => {
            let (rest, (align, offset)) = pair(leb128_u32, leb128_u32)(input)?;
            (rest, Instruction::I32Store(MemoryArg { align, offset }))
        }
        Opcode::I32Const => {
            let (rest, value) = leb128_i32(input)?;
            (rest, Instruction::I32Const(value))
        }
        Opcode::I32LtS => (input, Instruction::I32LtS),
        Opcode::I32Add => (input, Instruction::I32Add),
        Opcode::I32Sub => (input, Instruction::I32Sub),
    };
    Ok((rest, inst))
}

#[cfg(test)]
mod tests {
    use super::Module;

    #[test]
    fn decode_hello_world() -> anyhow::Result<()> {
        let wasm = wat::parse_str(include_str!("../fixtures/hello_world.wat"))?;
        let (_, module) = Module::new(&wasm).expect("failed to parse wasm");
        dbg!(module);
        Ok(())
    }

    #[test]
    fn decode_fib() -> anyhow::Result<()> {
        let wasm = wat::parse_str(include_str!("../fixtures/fib.wat"))?;
        let (_, module) = Module::new(&wasm).expect("failed to parse wasm");
        dbg!(module);

        Ok(())
    }
}
