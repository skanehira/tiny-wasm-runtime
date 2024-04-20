use super::{
    instruction::Instruction,
    opcode::Opcode,
    section::{Function, SectionCode},
    types::{
        Data, Export, ExportDesc, FuncType, FunctionLocal, Import, ImportDesc, Limits, Memory,
        ValueType,
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
    pub data_section: Option<Vec<Data>>,
    pub type_section: Option<Vec<FuncType>>,
    pub function_section: Option<Vec<u32>>,
    pub code_section: Option<Vec<Function>>,
    pub export_section: Option<Vec<Export>>,
    pub import_section: Option<Vec<Import>>,
}

impl Default for Module {
    fn default() -> Self {
        Self {
            magic: "\0asm".to_string(),
            version: 1,
            memory_section: None,
            data_section: None,
            type_section: None,
            function_section: None,
            code_section: None,
            export_section: None,
            import_section: None,
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

        while !remaining.is_empty() {
            match decode_section_header(remaining) {
                Ok((input, (code, size))) => {
                    let (rest, section_contents) = take(size)(input)?;

                    match code {
                        SectionCode::Custom => {
                            // skip
                        }
                        SectionCode::Memory => {
                            let (_, memory) = decode_memory_section(section_contents)?;
                            module.memory_section = Some(vec![memory]);
                        }
                        SectionCode::Data => {
                            let (_, data) = deocde_data_section(section_contents)?;
                            module.data_section = Some(data);
                        }
                        SectionCode::Type => {
                            let (_, types) = decode_type_section(section_contents)?;
                            module.type_section = Some(types);
                        }
                        SectionCode::Function => {
                            let (_, func_idx_list) = decode_function_section(section_contents)?;
                            module.function_section = Some(func_idx_list);
                        }
                        SectionCode::Code => {
                            let (_, funcs) = decode_code_section(section_contents)?;
                            module.code_section = Some(funcs);
                        }
                        SectionCode::Export => {
                            let (_, exports) = decode_export_section(section_contents)?;
                            module.export_section = Some(exports);
                        }
                        SectionCode::Import => {
                            let (_, imports) = decode_import_section(section_contents)?;
                            module.import_section = Some(imports);
                        }
                    };

                    remaining = rest;
                }
                Err(err) => return Err(err),
            }
        }
        Ok((input, module))
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

fn decode_vaue_type(input: &[u8]) -> IResult<&[u8], ValueType> {
    let (input, value_type) = le_u8(input)?;
    Ok((input, value_type.into()))
}

fn decode_type_section(input: &[u8]) -> IResult<&[u8], Vec<FuncType>> {
    let mut func_types: Vec<FuncType> = vec![];

    let (mut input, count) = leb128_u32(input)?;

    for _ in 0..count {
        let (rest, _) = le_u8(input)?;
        let mut func = FuncType::default();

        let (rest, size) = leb128_u32(rest)?;
        let (rest, types) = take(size)(rest)?;
        let (_, types) = many0(decode_vaue_type)(types)?;
        func.params = types;

        let (rest, size) = leb128_u32(rest)?;
        let (rest, types) = take(size)(rest)?;
        let (_, types) = many0(decode_vaue_type)(types)?;
        func.results = types;

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
        body.locals.push(FunctionLocal {
            type_count,
            value_type: value_type.into(),
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

fn decode_instructions(input: &[u8]) -> IResult<&[u8], Instruction> {
    let (input, byte) = le_u8(input)?;
    let op = Opcode::from_u8(byte).unwrap_or_else(|| panic!("invalid opcode: {:X}", byte));
    let (rest, inst) = match op {
        Opcode::LocalGet => {
            let (rest, idx) = leb128_u32(input)?;
            (rest, Instruction::LocalGet(idx))
        }
        Opcode::LocalSet => {
            let (rest, idx) = leb128_u32(input)?;
            (rest, Instruction::LocalSet(idx))
        }
        Opcode::I32Store => {
            let (rest, align) = leb128_u32(input)?;
            let (rest, offset) = leb128_u32(rest)?;
            (rest, Instruction::I32Store { align, offset })
        }
        Opcode::I32Const => {
            let (rest, value) = leb128_i32(input)?;
            (rest, Instruction::I32Const(value))
        }
        Opcode::I32Add => (input, Instruction::I32Add),
        Opcode::End => (input, Instruction::End),
        Opcode::Call => {
            let (rest, idx) = leb128_u32(input)?;
            (rest, Instruction::Call(idx))
        }
    };
    Ok((rest, inst))
}

fn decode_export_section(input: &[u8]) -> IResult<&[u8], Vec<Export>> {
    let (mut input, count) = leb128_u32(input)?;
    let mut exports = vec![];

    for _ in 0..count {
        let (rest, name) = decode_name(input)?;
        let (rest, export_kind) = le_u8(rest)?;
        let (rest, idx) = leb128_u32(rest)?;
        let desc = match export_kind {
            0x00 => ExportDesc::Func(idx),
            _ => unimplemented!("unsupported export kind: {:X}", export_kind),
        };
        exports.push(Export { name, desc });
        input = rest;
    }

    Ok((input, exports))
}

fn decode_import_section(input: &[u8]) -> IResult<&[u8], Vec<Import>> {
    let (mut input, count) = leb128_u32(input)?;
    let mut imports = vec![];

    for _ in 0..count {
        let (rest, module) = decode_name(input)?;
        let (rest, field) = decode_name(rest)?;
        let (rest, import_kind) = le_u8(rest)?;
        let (rest, desc) = match import_kind {
            0x00 => {
                let (rest, idx) = leb128_u32(rest)?;
                (rest, ImportDesc::Func(idx))
            }
            _ => unimplemented!("unsupported import kind: {:X}", import_kind),
        };

        imports.push(Import {
            module,
            field,
            desc,
        });

        input = rest;
    }

    Ok((&[], imports))
}

fn decode_memory_section(input: &[u8]) -> IResult<&[u8], Memory> {
    let (input, _) = leb128_u32(input)?;
    let (_, limits) = decode_limits(input)?;
    Ok((input, Memory { limits }))
}

fn decode_limits(input: &[u8]) -> IResult<&[u8], Limits> {
    let (input, (flags, min)) = pair(leb128_u32, leb128_u32)(input)?;
    let max = if flags == 0 {
        None
    } else {
        let (_, max) = leb128_u32(input)?;
        Some(max)
    };

    Ok((input, Limits { min, max }))
}

fn decode_expr(input: &[u8]) -> IResult<&[u8], u32> {
    let (input, _) = leb128_u32(input)?;
    let (input, offset) = leb128_u32(input)?;
    let (input, _) = leb128_u32(input)?;
    Ok((input, offset))
}

fn deocde_data_section(input: &[u8]) -> IResult<&[u8], Vec<Data>> {
    let (mut input, count) = leb128_u32(input)?;
    let mut data = vec![];
    for _ in 0..count {
        let (rest, memory_index) = leb128_u32(input)?;
        let (rest, offset) = decode_expr(rest)?;
        let (rest, size) = leb128_u32(rest)?;
        let (rest, init) = take(size)(rest)?;
        data.push(Data {
            memory_index,
            offset,
            init: init.into(),
        });
        input = rest;
    }
    Ok((input, data))
}

fn decode_name(input: &[u8]) -> IResult<&[u8], String> {
    let (input, size) = leb128_u32(input)?;
    let (input, name) = take(size)(input)?;
    Ok((
        input,
        String::from_utf8(name.to_vec()).expect("invalid utf-8 string"),
    ))
}

#[cfg(test)]
mod tests {
    use crate::binary::{
        instruction::Instruction,
        module::Module,
        section::Function,
        types::{
            Data, Export, ExportDesc, FuncType, FunctionLocal, Import, ImportDesc, Limits, Memory,
            ValueType,
        },
    };
    use anyhow::Result;

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
                type_section: Some(vec![FuncType::default()]),
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
    fn decode_func_param() -> Result<()> {
        let wasm = wat::parse_str("(module (func (param i32 i64)))")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32, ValueType::I64],
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
    fn decode_func_local() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_local.wat")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType::default()]),
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
                    code: vec![Instruction::End],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_func_add() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_add.wat")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32, ValueType::I32],
                    results: vec![ValueType::I32],
                }]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![
                        Instruction::LocalGet(0),
                        Instruction::LocalGet(1),
                        Instruction::I32Add,
                        Instruction::End
                    ],
                }]),
                export_section: Some(vec![Export {
                    name: "add".into(),
                    desc: ExportDesc::Func(0),
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_func_call() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/func_call.wat")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32],
                    results: vec![ValueType::I32],
                },]),
                function_section: Some(vec![0, 0]),
                code_section: Some(vec![
                    Function {
                        locals: vec![],
                        code: vec![
                            Instruction::LocalGet(0),
                            Instruction::Call(1),
                            Instruction::End
                        ],
                    },
                    Function {
                        locals: vec![],
                        code: vec![
                            Instruction::LocalGet(0),
                            Instruction::LocalGet(0),
                            Instruction::I32Add,
                            Instruction::End
                        ],
                    }
                ]),
                export_section: Some(vec![Export {
                    name: "call_doubler".into(),
                    desc: ExportDesc::Func(0),
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_import() -> Result<()> {
        let wasm = wat::parse_file("src/fixtures/import.wat")?;
        let module = Module::new(&wasm)?;
        assert_eq!(
            module,
            Module {
                type_section: Some(vec![FuncType {
                    params: vec![ValueType::I32],
                    results: vec![ValueType::I32],
                }]),
                import_section: Some(vec![Import {
                    module: "env".into(),
                    field: "add".into(),
                    desc: ImportDesc::Func(0),
                }]),
                export_section: Some(vec![Export {
                    name: "call_add".into(),
                    desc: ExportDesc::Func(1),
                }]),
                function_section: Some(vec![0]),
                code_section: Some(vec![Function {
                    locals: vec![],
                    code: vec![
                        Instruction::LocalGet(0),
                        Instruction::Call(0),
                        Instruction::End
                    ],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }

    #[test]
    fn decode_memory() -> Result<()> {
        let tests = vec![
            ("(module (memory 1))", Limits { min: 1, max: None }),
            (
                "(module (memory 1 2))",
                Limits {
                    min: 1,
                    max: Some(2),
                },
            ),
        ];
        for (wasm, limits) in tests {
            let module = Module::new(&wat::parse_str(wasm)?)?;
            assert_eq!(
                module,
                Module {
                    memory_section: Some(vec![Memory { limits }]),
                    ..Default::default()
                }
            );
        }
        Ok(())
    }

    #[test]
    fn decode_data() -> Result<()> {
        let tests = vec![
            (
                "(module (memory 1) (data (i32.const 0) \"hello\"))",
                vec![Data {
                    memory_index: 0,
                    offset: 0,
                    init: "hello".as_bytes().to_vec(),
                }],
            ),
            (
                "(module (memory 1) (data (i32.const 0) \"hello\") (data (i32.const 5) \"world\"))",
                vec![
                    Data {
                        memory_index: 0,
                        offset: 0,
                        init: b"hello".into(),
                    },
                    Data {
                        memory_index: 0,
                        offset: 5,
                        init: b"world".into(),
                    },
                ],
            ),
        ];

        for (wasm, data) in tests {
            let module = Module::new(&wat::parse_str(wasm)?)?;
            assert_eq!(
                module,
                Module {
                    memory_section: Some(vec![Memory {
                        limits: Limits { min: 1, max: None }
                    }]),
                    data_section: Some(data),
                    ..Default::default()
                }
            );
        }
        Ok(())
    }

    #[test]
    fn decode_i32_store() -> Result<()> {
        let wasm = wat::parse_str(
            "(module (func (i32.store offset=4 (i32.const 4))))",
        )?;
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
                    code: vec![
                        Instruction::I32Const(4),
                        Instruction::I32Store {
                            align: 2,
                            offset: 4
                        },
                        Instruction::End
                    ],
                }]),
                ..Default::default()
            }
        );
        Ok(())
    }
}
