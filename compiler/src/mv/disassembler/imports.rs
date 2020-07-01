use std::collections::BTreeMap;
use std::rc::Rc;
use libra::libra_vm::file_format::{
    StructFieldInformation, Kind, SignatureToken, StructHandleIndex, CompiledModuleMut, Signature,
};
use libra::libra_types::account_address::AccountAddress;
use crate::mv::disassembler::Encode;
use anyhow::Error;
use std::fmt::Write;

pub struct Imports<'a> {
    imports: BTreeMap<&'a str, BTreeMap<AccountAddress, Import<'a>>>,
}

impl<'a> Imports<'a> {
    pub fn new(module: &'a CompiledModuleMut) -> Imports<'a> {
        let mut imports = BTreeMap::new();

        for (index, handler) in module.module_handles.iter().enumerate() {
            if module.self_module_handle_idx.0 as usize != index {
                let module_name = module.identifiers[handler.name.0 as usize].as_str();
                let entry = imports.entry(module_name);
                let name_map = entry.or_insert_with(|| BTreeMap::new());
                let count = name_map.len();
                let address_entry =
                    name_map.entry(module.address_identifiers[handler.address.0 as usize]);
                address_entry.or_insert_with(|| {
                    if count == 0 {
                        Rc::new(ImportName::Name(module_name))
                    } else {
                        Rc::new(ImportName::Alias(module_name, count))
                    }
                });
            }
        }

        Imports { imports }
    }

    pub fn get_import(&self, address: &AccountAddress, name: &str) -> Option<Import<'a>> {
        self.imports
            .get(name)
            .and_then(|imports| imports.get(&address).map(|info| info.clone()))
    }
}

pub type Import<'a> = Rc<ImportName<'a>>;

#[derive(Debug)]
pub enum ImportName<'a> {
    Name(&'a str),
    Alias(&'a str, usize),
}

impl<'a> Encode for Imports<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
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
                if let ImportName::Alias(alias, id) = alias.as_ref() {
                    write!(w, " as {}_{}", alias, id)?;
                }
                writeln!(w, ";")?;
            }
        }
        Ok(())
    }
}

impl<'a> Encode for ImportName<'a> {
    fn encode<W: Write>(&self, w: &mut W, _: u8) -> Result<(), Error> {
        match &self {
            ImportName::Name(name) => w.write_str(name)?,
            ImportName::Alias(name, id) => write!(w, "{}_{}", name, id)?,
        }

        Ok(())
    }
}
