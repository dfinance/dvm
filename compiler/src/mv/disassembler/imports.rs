use std::collections::BTreeMap;
use std::rc::Rc;
use libra::prelude::*;
use crate::mv::disassembler::Encode;
use anyhow::Error;
use std::fmt::Write;
use crate::mv::disassembler::unit::UnitAccess;

/// Unit imports table.
#[derive(Debug)]
pub struct Imports<'a> {
    imports: BTreeMap<&'a str, BTreeMap<AccountAddress, Import<'a>>>,
}

impl<'a> Imports<'a> {
    /// Create a new imports table.
    pub fn new(unit: &'a impl UnitAccess) -> Imports<'a> {
        let mut imports = BTreeMap::new();

        let self_module_handle_idx = unit.self_module_handle_idx().map(|id| id.0 as usize);
        for (index, handler) in unit.module_handles().iter().enumerate() {
            if self_module_handle_idx != Some(index) {
                let module_name = unit.identifier(handler.name);
                let entry = imports.entry(module_name);
                let name_map = entry.or_insert_with(BTreeMap::new);
                let count = name_map.len();

                let address = *unit.address(handler.address);
                let address_entry = name_map.entry(address);
                address_entry.or_insert_with(|| {
                    if count == 0 {
                        Rc::new(ImportName::Name(address, module_name))
                    } else {
                        Rc::new(ImportName::Alias(address, module_name, count))
                    }
                });
            }
        }

        Imports { imports }
    }

    /// Returns import by address and module name.
    pub fn get_import(&self, address: &AccountAddress, name: &str) -> Option<Import<'a>> {
        self.imports
            .get(name)
            .and_then(|imports| imports.get(&address).cloned())
    }

    /// Returns `true` if the import contains no elements.
    pub fn is_empty(&self) -> bool {
        self.imports.is_empty()
    }
}

/// Import representation.
pub type Import<'a> = Rc<ImportName<'a>>;

/// Import name.
#[derive(Debug)]
pub enum ImportName<'a> {
    /// Simple module name.
    Name(AccountAddress, &'a str),
    /// Import alias.
    Alias(AccountAddress, &'a str, usize),
}

impl<'a> ImportName<'a> {
    /// Returns import address.
    pub fn address(&self) -> AccountAddress {
        match self {
            ImportName::Name(address, _) => *address,
            ImportName::Alias(address, _, _) => *address,
        }
    }
}

impl<'a> Encode for Imports<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        for (name, address_map) in &self.imports {
            for (address, alias) in address_map {
                write!(
                    w,
                    "{s:width$}use 0x{addr}::{name}",
                    s = "",
                    width = indent as usize,
                    addr = address,
                    name = name
                )?;
                if let ImportName::Alias(_, alias, id) = alias.as_ref() {
                    write!(w, " as {}_{}", alias, id)?;
                }
                writeln!(w, ";")?;
            }
        }
        Ok(())
    }
}

impl<'a> Encode for ImportName<'a> {
    fn encode<W: Write>(&self, w: &mut W, _: usize) -> Result<(), Error> {
        match &self {
            ImportName::Name(_, name) => w.write_str(name)?,
            ImportName::Alias(_, name, id) => write!(w, "{}_{}", name, id)?,
        }

        Ok(())
    }
}
