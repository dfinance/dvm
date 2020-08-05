use crate::mv::disassembler::code::exp::{Exp, SourceRange};
use crate::embedded::{AccountAddress, MoveValue, Bytecode};
use libra::file_format::ConstantPoolIndex;
use crate::mv::disassembler::Encode;
use anyhow::Error;
use std::fmt::Write;
use crate::mv::disassembler::unit::UnitAccess;

#[derive(Debug)]
pub enum Ld {
    U8(u8),
    U64(u64),
    U128(u128),
    Bool(bool),
    Address(AccountAddress),
    Vector(Vec<u8>),
}

impl Ld {
    pub fn u8<'a>(val: u8) -> Exp<'a> {
        Exp::Ld(Ld::U8(val))
    }

    pub fn u64<'a>(val: u64) -> Exp<'a> {
        Exp::Ld(Ld::U64(val))
    }

    pub fn u128<'a>(val: u128) -> Exp<'a> {
        Exp::Ld(Ld::U128(val))
    }

    pub fn bool<'a>(val: bool) -> Exp<'a> {
        Exp::Ld(Ld::Bool(val))
    }

    pub fn ld_const<'a>(index: ConstantPoolIndex, unit: &'a impl UnitAccess) -> Exp<'a> {
        let constant = &unit.constant(index);
        if let Some(constant) = constant.deserialize_constant() {
            match constant {
                MoveValue::Address(addr) => Exp::Ld(Ld::Address(addr)),
                MoveValue::Vector(v) => {
                    let val = v
                        .iter()
                        .map(|v| match v {
                            MoveValue::U8(v) => Some(*v),
                            _ => None,
                        })
                        .collect::<Option<Vec<u8>>>();

                    if let Some(val) = val {
                        Exp::Ld(Ld::Vector(val))
                    } else {
                        Exp::Error(Bytecode::LdConst(index))
                    }
                }
                _ => Exp::Error(Bytecode::LdConst(index)),
            }
        } else {
            Exp::Error(Bytecode::LdConst(index))
        }
    }
}

impl Encode for Ld {
    fn encode<W: Write>(&self, w: &mut W, _: usize) -> Result<(), Error> {
        match self {
            Ld::U8(val) => write!(w, "{}u8", val)?,
            Ld::U64(val) => write!(w, "{}", val)?,
            Ld::U128(val) => write!(w, "{}u128", val)?,
            Ld::Bool(val) => write!(w, "{}", val)?,
            Ld::Address(val) => write!(w, "0x{}", val)?,
            Ld::Vector(val) => write!(w, "x\"{}\"", hex::encode(&val))?,
        }
        Ok(())
    }
}

impl SourceRange for Ld {
    fn source_range(&self) -> Option<(usize, usize)> {
        None
    }
}
