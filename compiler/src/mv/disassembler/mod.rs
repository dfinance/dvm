use anyhow::Error;
use libra::libra_vm::CompiledModule;
use std::fmt::Write;
use crate::mv::disassembler::module::Module;
use crate::mv::disassembler::script::Script;
use libra::libra_vm::file_format::*;
use libra::move_core_types::language_storage::ModuleId;
use crate::mv::disassembler::generics::Generics;
use crate::mv::disassembler::imports::Imports;

mod code;
mod field;
mod functions;
mod generics;
mod imports;
mod module;
mod script;
mod structs;
mod types;

pub const INDENT: u8 = 4;

pub fn disasm<W: Write>(bytecode: &[u8], writer: &mut W) -> Result<(), Error> {
    let module = CompiledModule::deserialize(bytecode)?;

    let id = module.self_id();
    let inner = module.as_inner();

    let mut imports = Imports::new(inner);
    let mut generic_handler = Generics::new(inner);

    let unit = Unit::new(&id, inner, &mut imports, &mut generic_handler)?;
    unit.write_code(writer)
}

pub fn disasm_str(bytecode: &[u8]) -> Result<String, Error> {
    let mut code = String::new();
    disasm(bytecode, &mut code)?;
    Ok(code)
}

pub enum Unit<'a> {
    Script(Script),
    Module(Module<'a>),
}

impl<'a> Unit<'a> {
    pub fn new(
        id: &'a ModuleId,
        module: &'a CompiledModuleMut,
        imports: &'a Imports<'a>,
        generics: &'a Generics,
    ) -> Result<Unit<'a>, Error> {
        //todo implemets script case.
        Ok(Unit::Module(Module::new(id, module, imports, generics)))
    }

    pub fn write_code<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            Unit::Script(script) => script.encode(writer, 0),
            Unit::Module(module) => module.encode(writer, 0),
        }
    }

    pub fn code_string(&self) -> Result<String, Error> {
        let mut code = String::new();
        self.write_code(&mut code)?;
        Ok(code)
    }
}

pub trait Encode {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error>;
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
    use libra::move_core_types::language_storage::CORE_CODE_ADDRESS;
    use libra::libra_vm::CompiledModule;
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
        assert_eq!(
            original_bytecode,
            CompiledModule::deserialize(&restored_bytecode).unwrap()
        );
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

    #[test]
    pub fn test_if() {
        perform_test(include_str!("assets/code/if.move"));
    }
}
