use crate::mv::disassembler::imports::Imports;
use crate::mv::disassembler::generics::Generic;
use std::fmt::Write;
use anyhow::Error;
use crate::mv::disassembler::Encode;
use crate::disassembler::functions::Param;
use std::sync::atomic::{Ordering, AtomicBool};
use std::rc::Rc;
use libra::libra_vm::file_format::*;
use crate::mv::disassembler::types::{FType, extract_type_signature};

#[derive(Debug)]
pub struct Locals<'a> {
    pub inner: Vec<Local<'a>>,
}

impl<'a> Locals<'a> {
    pub fn new(
        params: &[Param<'a>],
        module: &'a CompiledModuleMut,
        imports: &'a Imports,
        type_params: &[Generic],
        sign: &'a Signature,
    ) -> Locals<'a> {
        let locals = params
            .iter()
            .map(|p| Local::Param(p.clone()))
            .chain(
                sign.0
                    .iter()
                    .map(|t| extract_type_signature(module, t, imports, type_params))
                    .enumerate()
                    .map(|(index, t)| {
                        Local::Var(Var {
                            used: Rc::new(AtomicBool::new(false)),
                            index,
                            f_type: Rc::new(t),
                        })
                    }),
            )
            .collect();

        Locals { inner: locals }
    }

    pub fn get(&self, index: usize) -> Local<'a> {
        self.inner[index].clone()
    }
}

#[derive(Debug, Clone)]
pub struct Var<'a> {
    used: Rc<AtomicBool>,
    index: usize,
    f_type: Rc<FType<'a>>,
}

impl<'a> Var<'a> {
    pub fn mark_as_used(&self) {
        self.used.store(true, Ordering::Relaxed);
    }

    pub fn write_name<W: Write>(&self, w: &mut W) -> Result<(), Error> {
        if !self.used.load(Ordering::Relaxed) {
            w.write_str("_")?;
        }
        w.write_str("var")?;

        if self.index != 0 {
            write!(w, "{}", self.index)?;
        }
        Ok(())
    }
}

impl<'a> Encode for Var<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        self.write_name(w)?;
        w.write_str(": ")?;
        self.f_type.encode(w, indent)
    }
}

#[derive(Debug, Clone)]
pub enum Local<'a> {
    Param(Param<'a>),
    Var(Var<'a>),
}

impl<'a> Local<'a> {
    pub fn mark_as_used(&self) {
        match self {
            Local::Param(p) => p.mark_as_used(),
            Local::Var(v) => v.mark_as_used(),
        }
    }

    pub fn write_name<W: Write>(&self, w: &mut W) -> Result<(), Error> {
        match self {
            Local::Param(p) => p.write_name(w),
            Local::Var(v) => v.write_name(w),
        }
    }
}

impl<'a> Encode for Local<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        match &self {
            Local::Param(p) => p.write_name(w),
            Local::Var(v) => v.write_name(w),
        }
    }
}
