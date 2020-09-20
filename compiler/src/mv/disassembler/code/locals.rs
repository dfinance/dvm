use std::rc::Rc;
use std::fmt::Write;
use std::sync::atomic::{Ordering, AtomicBool};
use anyhow::Error;
use serde::{Serialize, Deserialize};
use libra::file_format::*;
use crate::mv::disassembler::imports::Imports;
use crate::mv::disassembler::generics::Generic;
use crate::mv::disassembler::Encode;
use crate::disassembler::functions::Param;
use crate::mv::disassembler::types::{FType, extract_type_signature};
use crate::mv::disassembler::unit::UnitAccess;

/// Local variable representation.
#[derive(Debug, Serialize, Deserialize)]
pub struct Locals<'a> {
    /// Local variables.
    #[serde(borrow)]
    pub inner: Vec<Local<'a>>,
}

impl<'a> Locals<'a> {
    /// Create a new local variables.
    pub fn new(
        params: &[Param<'a>],
        unit: &'a impl UnitAccess,
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
                    .map(|t| extract_type_signature(unit, t, imports, type_params))
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

    /// Returns local variables by its index.
    pub fn get(&self, index: usize) -> Local<'a> {
        self.inner[index].clone()
    }

    /// Returns the empty locals variables list.
    /// Used for light disassembler version.
    pub fn mock() -> Locals<'static> {
        Locals { inner: vec![] }
    }
}

/// Variable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Var<'a> {
    used: Rc<AtomicBool>,
    index: usize,
    #[serde(borrow)]
    #[serde(deserialize_with = "FType::deserialize_rc")]
    f_type: Rc<FType<'a>>,
}

impl<'a> Var<'a> {
    /// Makes variable as used.
    pub fn mark_as_used(&self) {
        self.used.store(true, Ordering::Relaxed);
    }

    /// Writes variable name to the given writer.
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
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        self.write_name(w)?;
        w.write_str(": ")?;
        self.f_type.encode(w, indent)
    }
}

/// Local variable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Local<'a> {
    /// Function parameters.
    #[serde(borrow)]
    Param(Param<'a>),
    /// Variable.
    #[serde(borrow)]
    Var(Var<'a>),
}

impl<'a> Local<'a> {
    /// Makes local variable as used.
    pub fn mark_as_used(&self) {
        match self {
            Local::Param(p) => p.mark_as_used(),
            Local::Var(v) => v.mark_as_used(),
        }
    }

    /// Writes local variable name to the given writer.
    pub fn write_name<W: Write>(&self, w: &mut W) -> Result<(), Error> {
        match self {
            Local::Param(p) => p.write_name(w),
            Local::Var(v) => v.write_name(w),
        }
    }
}

impl<'a> Encode for Local<'a> {
    fn encode<W: Write>(&self, w: &mut W, _: usize) -> Result<(), Error> {
        match &self {
            Local::Param(p) => p.write_name(w),
            Local::Var(v) => v.write_name(w),
        }
    }
}
