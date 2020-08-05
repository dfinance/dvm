use anyhow::Error;
use std::fmt::Write;
use crate::mv::disassembler::unit::{CompiledUnit as Unit, Disassembler};

pub mod code;
pub mod field;
pub mod functions;
pub mod generics;
pub mod imports;
pub mod module;
pub mod script;
pub mod structs;
pub mod types;
pub mod unit;

pub const INDENT: usize = 4;

pub fn disasm<W: Write>(bytecode: &[u8], writer: &mut W) -> Result<(), Error> {
    let unit = Unit::new(bytecode)?;
    let disasm = Disassembler::new(&unit);
    let ast = disasm.as_source_unit();
    ast.write_code(writer)
}

pub fn disasm_str(bytecode: &[u8]) -> Result<String, Error> {
    let mut code = String::new();
    disasm(bytecode, &mut code)?;
    Ok(code)
}

pub trait Encode {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error>;
}

pub fn write_array<E: Encode, W: Write>(
    w: &mut W,
    prefix: &str,
    decimeter: &str,
    encode: &[E],
    suffix: &str,
) -> Result<(), Error> {
    w.write_str(prefix)?;
    for (index, e) in encode.iter().enumerate() {
        e.encode(w, 0)?;
        if index != encode.len() - 1 {
            w.write_str(decimeter)?;
        }
    }
    w.write_str(suffix)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::Compiler;
    use ds::MockDataSource;
    use libra::prelude::*;
    use libra::file_format::*;
    use crate::mv::disassembler::disasm_str;

    pub fn perform_test(source: &str) {
        let ds = MockDataSource::new();
        let compiler = Compiler::new(ds.clone());
        ds.publish_module(
            compiler
                .compile(include_str!("assets/base.move"), Some(CORE_CODE_ADDRESS))
                .unwrap(),
        )
        .unwrap();

        ds.publish_module(
            compiler
                .compile(include_str!("assets/tx.move"), Some(CORE_CODE_ADDRESS))
                .unwrap(),
        )
        .unwrap();

        let original_bytecode = compiler.compile(source, Some(CORE_CODE_ADDRESS)).unwrap();
        let restored_source = disasm_str(&original_bytecode).unwrap();
        println!("{}", restored_source);

        let original_bytecode = CompiledModule::deserialize(&original_bytecode).unwrap();
        let restored_bytecode = compiler
            .compile(&restored_source, Some(CORE_CODE_ADDRESS))
            .unwrap();

        compare_bytecode(
            original_bytecode,
            CompiledModule::deserialize(&restored_bytecode).unwrap(),
        );
    }

    fn compare_bytecode(expected: CompiledModule, actual: CompiledModule) {
        let mut expected = expected.into_inner();
        let mut actual = actual.into_inner();

        fn normalize_bytecode(bytecode: &mut CodeUnit) {
            bytecode.code = bytecode
                .code
                .iter()
                .cloned()
                .map(|mut bc| {
                    if let Bytecode::MoveLoc(i) = &bc {
                        bc = Bytecode::CopyLoc(*i);
                    }

                    bc
                })
                .collect();
        }

        fn normalize_f_def(func_def: &mut [FunctionDefinition]) {
            for def in func_def {
                if let Some(code) = &mut def.code {
                    normalize_bytecode(code);
                }
            }
        }

        normalize_f_def(&mut expected.function_defs);
        normalize_f_def(&mut actual.function_defs);

        assert_eq!(expected, actual);
    }

    #[test]
    pub fn test_script() {
        perform_test(include_str!("assets/script.move"));
    }

    #[test]
    pub fn test_empty_module() {
        perform_test(include_str!("assets/empty.move"));
    }

    #[test]
    pub fn test_simple_struct() {
        perform_test(include_str!("assets/struct.move"));
    }

    #[test]
    pub fn test_function_signature() {
        perform_test(include_str!("assets/function_sign.move"));
    }

    #[test]
    pub fn test_abort() {
        perform_test(include_str!("assets/code/abort.move"));
    }

    #[test]
    pub fn test_call() {
        perform_test(include_str!("assets/code/call.move"));
    }

    #[test]
    pub fn test_arithmetic() {
        perform_test(include_str!("assets/code/arithmetic.move"));
    }

    #[test]
    pub fn test_values() {
        perform_test(include_str!("assets/code/values.move"));
    }

    #[test]
    pub fn test_fake_native() {
        perform_test(include_str!("assets/code/fake_native.move"));
    }

    #[test]
    pub fn test_let() {
        perform_test(include_str!("assets/code/let.move"));
    }

    #[test]
    pub fn test_pack() {
        perform_test(include_str!("assets/code/pack.move"));
    }

    #[test]
    pub fn test_unpack() {
        perform_test(include_str!("assets/code/unpack.move"));
    }

    #[test]
    pub fn test_loc() {
        perform_test(include_str!("assets/code/loc.move"));
    }

    #[ignore]
    #[test]
    pub fn test_loop() {
        perform_test(include_str!("assets/code/loop.move"));
    }

    #[ignore]
    #[test]
    pub fn test_while() {
        perform_test(include_str!("assets/code/while.move"));
    }

    #[ignore]
    #[test]
    pub fn test_if() {
        perform_test(include_str!("assets/code/if.move"));
    }
}
