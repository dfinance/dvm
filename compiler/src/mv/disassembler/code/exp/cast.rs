use crate::mv::disassembler::code::exp::{ExpLoc, Exp, SourceRange};
use crate::mv::disassembler::code::translator::Context;
use crate::mv::disassembler::Encode;
use std::fmt::Write;
use anyhow::Error;

#[derive(Debug)]
pub struct Cast<'a> {
    exp: ExpLoc<'a>,
    ty: CastType,
}

impl<'a> Cast<'a> {
    pub fn new(ty: CastType, ctx: &mut impl Context<'a>) -> Exp<'a> {
        Exp::Cast(Cast {
            exp: ctx.pop_exp(),
            ty,
        })
    }
}

impl<'a> SourceRange for Cast<'a> {
    fn source_range(&self) -> Option<(usize, usize)> {
        self.exp.source_range()
    }
}

impl<'a> Encode for Cast<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        w.write_str("(")?;
        self.exp.encode(w, indent)?;
        w.write_str(" as ")?;
        self.ty.encode(w, indent)?;
        w.write_str(")")?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum CastType {
    U8,
    U64,
    U128,
}

impl Encode for CastType {
    fn encode<W: Write>(&self, w: &mut W, _: usize) -> Result<(), Error> {
        w.write_str(match self {
            CastType::U8 => "u8",
            CastType::U64 => "u64",
            CastType::U128 => "u128",
        })?;
        Ok(())
    }
}
