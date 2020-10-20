use crate::mv::disassembler::{Encode, INDENT, Config};
use anyhow::Error;
use crate::mv::disassembler::imports::Imports;
use crate::mv::disassembler::generics::Generics;
use crate::mv::disassembler::functions::FunctionsDef;
use crate::mv::disassembler::unit::{UnitAccess};
use std::fmt::Write;

/// Script representation.
pub struct Script<'a> {
    imports: &'a Imports<'a>,
    function: FunctionsDef<'a>,
}

impl<'a> Script<'a> {
    /// Creates a new script.
    pub fn new(
        unit: &'a impl UnitAccess,
        imports: &'a Imports<'a>,
        generics: &'a Generics,
        _cfg: &Config,
    ) -> Script<'a> {
        let main = FunctionsDef::script(unit, &imports, &generics);
        Script {
            imports,
            function: main,
        }
    }

    /// Returns function definition.
    pub fn function(&self) -> &FunctionsDef<'a> {
        &self.function
    }
}

impl<'a> Encode for Script<'a> {
    fn encode<W: Write>(&self, w: &mut W, _: usize) -> Result<(), Error> {
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
