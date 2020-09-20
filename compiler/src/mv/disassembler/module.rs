use std::fmt::Write;
use anyhow::Error;
use libra::prelude::*;
use serde::{Serialize, Deserialize};
use crate::disassembler::structs::StructDef;
use crate::mv::disassembler::{Encode, INDENT, Config};
use crate::mv::disassembler::generics::Generics;
use crate::mv::disassembler::imports::Imports;
use crate::mv::disassembler::functions::FunctionsDef;
use crate::mv::disassembler::unit::{UnitAccess};

/// Module representation.
#[derive(Debug, Serialize, Deserialize)]
pub struct Module<'a> {
    address: Option<AccountAddress>,
    name: String,
    #[serde(borrow)]
    structs: Vec<StructDef<'a>>,
    functions: Vec<FunctionsDef<'a>>,
    imports: &'a Imports<'a>,
}

impl<'a> Module<'a> {
    /// Creates a new module.
    pub fn new(
        unit: &'a impl UnitAccess,
        imports: &'a Imports<'a>,
        generics: &'a Generics,
        config: &'a Config,
    ) -> Module<'a> {
        let structs = unit
            .struct_defs()
            .iter()
            .map(|def| StructDef::new(def, unit, generics, imports, config))
            .collect();

        let functions = unit
            .function_defs()
            .iter()
            .map(|def| FunctionsDef::new(def, unit, generics, imports, config))
            .collect();

        let id = unit.self_id();
        Module {
            address: Some(*id.address()),
            name: id.name().as_str().to_owned(),
            structs,
            functions,
            imports,
        }
    }
}

impl<'a> Encode for Module<'a> {
    fn encode<W: Write>(&self, w: &mut W, _indent: usize) -> Result<(), Error> {
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

        if self.address.is_some() {
            writeln!(w, "}}")?;
        }
        Ok(())
    }
}
