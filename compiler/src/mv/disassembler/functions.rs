use libra::libra_vm::file_format::*;
use crate::mv::disassembler::imports::Imports;
use crate::mv::disassembler::generics::{Generics, Generic, extract_type_params, write_type_parameters};
use crate::mv::disassembler::{Encode, write_array, INDENT};
use anyhow::Error;
use std::fmt::Write;
use crate::mv::disassembler::types::{
    FType, extract_type_signature, FullStructName, extract_struct_name,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::rc::Rc;
use crate::mv::disassembler::code::Body;

pub struct FunctionsDef<'a> {
    is_public: bool,
    is_native: bool,
    name: &'a str,
    type_params: Vec<Generic>,
    ret: Vec<FType<'a>>,
    params: Vec<Param<'a>>,
    acquires: Vec<FullStructName<'a>>,
    body: Option<Body<'a>>,
}

impl<'a> FunctionsDef<'a> {
    pub fn new(
        def: &'a FunctionDefinition,
        module: &'a CompiledModuleMut,
        generics: &'a Generics,
        imports: &'a Imports,
    ) -> FunctionsDef<'a> {
        let handler = &module.function_handles[def.function.0 as usize];
        let name = module.identifiers[handler.name.0 as usize].as_str();
        let type_params = extract_type_params(&handler.type_parameters, generics);

        let ret = module.signatures[handler.return_.0 as usize]
            .0
            .iter()
            .map(|tkn| extract_type_signature(module, tkn, imports, &type_params))
            .collect::<Vec<_>>();

        let params = module.signatures[handler.parameters.0 as usize]
            .0
            .iter()
            .enumerate()
            .map(|(index, tkn)| Param {
                used: Rc::new(AtomicBool::new(false)),
                index,
                f_type: Rc::new(extract_type_signature(module, tkn, imports, &type_params)),
            })
            .collect::<Vec<_>>();

        let body = def
            .code
            .as_ref()
            .map(|code| Body::new(code, ret.len(), module, &params, &imports, &type_params));

        let acquires = def
            .acquires_global_resources
            .iter()
            .map(|di| {
                let struct_defs = &module.struct_defs[di.0 as usize];
                extract_struct_name(module, &struct_defs.struct_handle, imports)
            })
            .collect();

        FunctionsDef {
            is_public: def.is_public(),
            is_native: def.is_native(),
            name,
            type_params,
            ret,
            params,
            acquires,
            body,
        }
    }
}

impl<'a> Encode for FunctionsDef<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        write!(
            w,
            "{s:width$}{native}{p}fun {name}",
            s = "",
            width = indent as usize,
            p = if self.is_public { "public " } else { "" },
            native = if self.is_native { "native " } else { "" },
            name = self.name,
        )?;
        write_type_parameters(w, &self.type_params)?;

        write_array(w, "(", ", ", &self.params, ")")?;

        if !self.ret.is_empty() {
            w.write_str(": ")?;
            if self.ret.len() == 1 {
                self.ret[0].encode(w, 0)?;
            } else {
                write_array(w, "(", ", ", &self.ret, ")")?;
            }
        }

        if !self.acquires.is_empty() {
            write_array(w, " acquires ", ", ", &self.acquires, " ")?;
        }

        if self.is_native {
            w.write_str(";")?;
        } else {
            w.write_str(" {")?;
            if let Some(body) = self.body.as_ref() {
                body.encode(w, indent + INDENT)?;
            }
            write!(w, "\n{s:width$}}}", s = "", width = indent as usize)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Param<'a> {
    used: Rc<AtomicBool>,
    index: usize,
    f_type: Rc<FType<'a>>,
}

impl<'a> Param<'a> {
    pub fn mark_as_used(&self) {
        self.used.store(true, Ordering::Relaxed);
    }

    pub fn write_name<W: Write>(&self, w: &mut W) -> Result<(), Error> {
        if !self.used.load(Ordering::Relaxed) {
            w.write_str("_")?;
        }
        w.write_str("arg")?;

        if self.index != 0 {
            write!(w, "{}", self.index)?;
        }
        Ok(())
    }
}

impl<'a> Encode for Param<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        self.write_name(w)?;
        w.write_str(": ")?;
        self.f_type.encode(w, indent)
    }
}
