use crate::disassembler::structs::StructDef;
use libra::libra_types::account_address::AccountAddress;
use anyhow::Error;
use crate::mv::disassembler::{Encode, INDENT};
use std::convert::TryFrom;
use libra::libra_vm::CompiledModule;
use libra::libra_vm::file_format::{
    StructFieldInformation, Kind, SignatureToken, StructHandleIndex, CompiledModuleMut, Signature,
};
use std::fmt::Write;
use crate::mv::disassembler::generics::Generics;
use libra::move_core_types::language_storage::ModuleId;
use crate::mv::disassembler::imports::Imports;
use crate::mv::disassembler::functions::FunctionsDef;

pub struct Module<'a> {
    address: Option<AccountAddress>,
    name: &'a str,
    structs: Vec<StructDef<'a>>,
    functions: Vec<FunctionsDef<'a>>,
    imports: &'a Imports<'a>,
}

impl<'a> Module<'a> {
    pub fn new(
        id: &'a ModuleId,
        module: &'a CompiledModuleMut,
        imports: &'a Imports<'a>,
        generics: &'a Generics,
    ) -> Module<'a> {
        let structs = module
            .struct_defs
            .iter()
            .map(|def| StructDef::new(def, &module, generics, imports))
            .collect();

        let functions = module
            .function_defs
            .iter()
            .map(|def| FunctionsDef::new(def, &module, generics, imports))
            .collect();

        Module {
            address: Some(*id.address()),
            name: id.name().as_str(),
            structs,
            functions,
            imports,
        }
    }
}

impl<'a> Encode for Module<'a> {
    fn encode<W: Write>(&self, w: &mut W, _indent: u8) -> Result<(), Error> {
        if let Some(address) = self.address {
            writeln!(w, "address 0x{} {{ ", address)?;
        }

        writeln!(w, "module {} {{", self.name)?;

        self.imports.encode(w, INDENT)?;
        writeln!(w)?;

        for struct_def in &self.structs {
            struct_def.encode(w, INDENT)?;
            writeln!(w, "\n")?;
        }

        for function in &self.functions {
            function.encode(w, INDENT)?;
            writeln!(w, "\n")?;
        }

        writeln!(w, "}}")?;

        if let Some(_) = self.address {
            writeln!(w, "}}")?;
        }
        Ok(())
    }
}
