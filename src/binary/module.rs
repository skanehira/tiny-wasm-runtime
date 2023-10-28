use super::{
    instruction::Instruction,
    opcode::Opcode,
    section::{Data, Export, Function, Import, Memory, SectionCode},
    types::{
        Block, BlockType, ExportKind, Expr, ExprValue, FuncType, FunctionLocal, ImportKind, Limits,
        MemoryArg, ValueType,
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

#[derive(Debug, PartialEq, Eq)]
pub struct Module {
    pub magic: String,
    pub version: u32,
    pub memory_section: Option<Vec<Memory>>,
    pub type_section: Option<Vec<FuncType>>,
    pub function_section: Option<Vec<u32>>,
    pub import_section: Option<Vec<Import>>,
    pub export_section: Option<Vec<Export>>,
    pub data_section: Option<Vec<Data>>,
    pub code_section: Option<Vec<Function>>,
}

impl Default for Module {
    fn default() -> Self {
        Self {
            magic: "\0asm".into(),
            version: 1,
            memory_section: None,
            type_section: None,
            function_section: None,
            import_section: None,
            export_section: None,
            data_section: None,
            code_section: None,
        }
    }
}

impl Module {
    pub fn new(input: &[u8]) -> anyhow::Result<Module> {
        let (_, module) =
            Module::decode(input).map_err(|e| anyhow::anyhow!("failed to parse wasm: {}", e))?;
        Ok(module)
    }

    fn decode(input: &[u8]) -> IResult<&[u8], Module> {
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
                // (code, size): セクションコードとサイズ
                Ok((input, (code, size))) => {
                    // セクションのサイズだけバイト列を切り出す
                    // rest は残りのバイト列
                    let (rest, section_contents) = take(size)(input)?;

                    match code {
                        SectionCode::Memory => {
                            let (_, memory) = decode_memory_section(section_contents)?;
                            module.memory_section = Some(vec![memory]);
                        }
                        SectionCode::Type => {
                            let (_, func_types) = decode_type_section(section_contents)?;
                            module.type_section = Some(func_types);
                        }
                        SectionCode::Function => {
                            let (_, func_idx_list) = decode_function_section(section_contents)?;
                            module.function_section = Some(func_idx_list);
                        }
                        SectionCode::Import => {
                            let (_, imports) = decode_import_section(section_contents)?;
                            module.import_section = Some(imports);
                        }
                        SectionCode::Export => {
                            let (_, exports) = decode_export_section(section_contents)?;
                            module.export_section = Some(exports);
                        }
                        SectionCode::Data => {
                            let (_, data) = decode_data_section(section_contents)?;
                            module.data_section = Some(data);
                        }
                        SectionCode::Code => {
                            let (_, funcs) = decode_code_section(section_contents)?;
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

fn decode_section_header(input: &[u8]) -> IResult<&[u8], (SectionCode, u32)> {
    let (input, (code, size)) = pair(le_u8, leb128_u32)(input)?;
    Ok((
        input,
        (
            SectionCode::from_u8(code).expect("unexpected section code"),
            size,
        ),
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
            _ => unreachable!(),
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
            _ => unreachable!(),
        };

        exports.push(Export { name, kind });

        input = rest;
    }

    Ok((&[], exports))
}

fn decode_limits(input: &[u8]) -> IResult<&[u8], Limits> {
    let (input, (limits, min)) = pair(leb128_u32, leb128_u32)(input)?;
    let max = if limits == 0 {
        None
    } else {
        let (_, max) = leb128_u32(input)?;
        Some(max)
    };

    Ok((&[], Limits { min, max }))
}

fn decode_type_section(input: &[u8]) -> IResult<&[u8], Vec<FuncType>> {
    let mut func_types: Vec<FuncType> = vec![];

    // countは関数の型定義の数
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
        unimplemented!();
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
    use crate::{
        binary::{
            section::{Data, Export, Function, Import, Memory},
            types::{
                Block, BlockType, ExportKind, FuncType, FunctionLocal, ImportKind, Limits,
                MemoryArg, ValueType,
            },
        },
        Instruction, Module,
    };
    use anyhow::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn decode_simplest_module() -> Result<()> {
        let wasm = wat::parse_str("(module)")?;
        let module = Module::new(&wasm)?;
        assert_eq!(module, Module::default());
        Ok(())
    }

    #[test]
    fn decode_simplest_func() -> Result<()> {
        let wasm = wat::parse_str("(module (func))")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![],
                    results: vec![],
                }]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![Instruction::End],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_memory() -> Result<()> {
        let wasm = wat::parse_str("(module (memory 1))")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                memory_section: Some(vec![Memory {
                    limits: Limits { min: 1, max: None }
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_data() -> Result<()> {
        let wasm = wat::parse_str(include_str!("../fixtures/data.wat"))?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                memory_section: Some(vec![Memory {
                    limits: Limits { min: 1, max: None }
                }]),
                data_section: Some(vec![Data {
                    memory_idx: 0,
                    init: vec![0x68, 0x65, 0x6c, 0x6c, 0x6f],
                    offset: 0.into(),
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_import() -> Result<()> {
        let wasm = wat::parse_str(include_str!("../fixtures/import.wat"))?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![],
                    results: vec![],
                }]),
                import_section: Some(vec![Import {
                    module: "module".into(),
                    field: "func".into(),
                    kind: ImportKind::Func(0),
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_export() -> Result<()> {
        let wasm = wat::parse_str(include_str!("../fixtures/export.wat"))?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![],
                    results: vec![],
                }]),
                function_section: Some(vec![0]),
                export_section: Some(vec![Export {
                    name: "dummy".into(),
                    kind: ExportKind::Func(0),
                }]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![Instruction::End],
                },]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_func_param_result() -> Result<()> {
        let wasm = wat::parse_str(include_str!("../fixtures/func_param_result.wat"))?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32, ValueType::I32],
                    results: vec![ValueType::I32, ValueType::I32],
                }]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![
                        Instruction::LocalGet(0),
                        Instruction::LocalGet(1),
                        Instruction::End,
                    ],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_func_local() -> Result<()> {
        let wasm = wat::parse_str(include_str!("../fixtures/func_local.wat"))?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![],
                    results: vec![],
                }]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![
                        FunctionLocal {
                            type_count: 1,
                            value_type: ValueType::I32,
                        },
                        FunctionLocal {
                            type_count: 2,
                            value_type: ValueType::I64,
                        },
                    ],
                    code: vec![Instruction::End,],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_call_func() -> Result<()> {
        let wasm = wat::parse_str(include_str!("../fixtures/call.wat"))?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![],
                    results: vec![],
                }]),
                function_section: Some(vec![0, 0]),
                code_section: Some(vec![
                    Function {
                        locals: vec![],
                        code: vec![Instruction::End,],
                    },
                    Function {
                        locals: vec![],
                        code: vec![Instruction::Call(0), Instruction::End,],
                    },
                ]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_fib() -> Result<()> {
        let wasm = wat::parse_str(include_str!("../fixtures/fib.wat"))?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32],
                    results: vec![ValueType::I32],
                }]),
                function_section: Some(vec![0]),
                export_section: Some(vec![Export {
                    name: "fib".into(),
                    kind: ExportKind::Func(0),
                }]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![
                        Instruction::LocalGet(0),
                        Instruction::I32Const(2),
                        Instruction::I32LtS,
                        Instruction::If(Block {
                            block_type: BlockType::Empty
                        }),
                        Instruction::I32Const(1),
                        Instruction::Return,
                        Instruction::End,
                        Instruction::LocalGet(0),
                        Instruction::I32Const(2),
                        Instruction::I32Sub,
                        Instruction::Call(0),
                        Instruction::LocalGet(0),
                        Instruction::I32Const(1),
                        Instruction::I32Sub,
                        Instruction::Call(0),
                        Instruction::I32Add,
                        Instruction::Return,
                        Instruction::End,
                    ],
                }]),
                ..Default::default()
            }
        );

        Ok(())
    }

    #[test]
    fn decode_hello_world() -> Result<()> {
        let wasm = wat::parse_str(include_str!("../fixtures/hello_world.wat"))?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                memory_section: Some(vec![Memory {
                    limits: Limits { min: 1, max: None }
                }]),
                type_section: Some(vec![
                    FuncType {
                        params: vec![
                            ValueType::I32,
                            ValueType::I32,
                            ValueType::I32,
                            ValueType::I32
                        ],
                        results: vec![ValueType::I32],
                    },
                    FuncType {
                        params: vec![],
                        results: vec![ValueType::I32],
                    }
                ]),
                function_section: Some(vec![1]),
                import_section: Some(vec![Import {
                    module: "wasi_snapshot_preview1".into(),
                    field: "fd_write".into(),
                    kind: ImportKind::Func(0),
                }]),
                export_section: Some(vec![Export {
                    name: "_start".into(),
                    kind: ExportKind::Func(1),
                }]),
                data_section: Some(vec![Data {
                    memory_idx: 0,
                    offset: 0.into(),
                    init: vec![
                        0x48, 0x65, 0x6c, 0x6c, 0x6f, // "Hello"
                        0x2c, 0x20, // ", "
                        0x57, 0x6f, 0x72, 0x6c, 0x64, // "World"
                        0x21, 0x0a // "!\n"
                    ],
                }]),
                code_section: Some(vec![Function {
                    locals: vec![FunctionLocal {
                        type_count: 1,
                        value_type: ValueType::I32,
                    }],
                    code: vec![
                        Instruction::I32Const(16),
                        Instruction::I32Const(0),
                        Instruction::I32Store(MemoryArg {
                            align: 2,
                            offset: 0,
                        }),
                        Instruction::I32Const(20),
                        Instruction::I32Const(14),
                        Instruction::I32Store(MemoryArg {
                            align: 2,
                            offset: 0,
                        }),
                        Instruction::I32Const(16),
                        Instruction::LocalSet(0),
                        Instruction::I32Const(1),
                        Instruction::LocalGet(0),
                        Instruction::I32Const(1),
                        Instruction::I32Const(24),
                        Instruction::Call(0),
                        Instruction::End,
                    ],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }
}
