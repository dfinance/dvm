use crate::mv::disassembler::{Encode, INDENT};
use anyhow::Error;
use crate::mv::disassembler::imports::Imports;
use crate::mv::disassembler::generics::Generics;
use crate::mv::disassembler::functions::FunctionsDef;
use libra::file_format::*;
use crate::mv::disassembler::unit::UnitAccess;
use std::fmt::Write;

pub struct Script<'a> {
    imports: &'a Imports<'a>,
    function: FunctionsDef<'a>,
}

impl<'a> Script<'a> {
    pub fn new(
        unit: &'a impl UnitAccess,
        imports: &'a Imports<'a>,
        generics: &'a Generics,
    ) -> Script<'a> {
        let main = FunctionsDef::script(unit, &imports, &generics);
        Script {
            imports,
            function: main,
        }
    }
}

impl<'a> Encode for Script<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        writeln!(w, "script {{")?;
        self.imports.encode(w, INDENT)?;
        if !self.imports.is_empty() {
            writeln!(w)?;
        }

        self.function.encode(w, INDENT)?;
        writeln!(w, "}}")?;
        Ok(())
    }
}
