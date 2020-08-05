pub mod block;
pub mod branching;
pub mod cast;
pub mod function;
pub mod ld;
pub mod loc;
pub mod lt;
pub mod operators;
pub mod pack;
pub mod ret;
pub mod rf;
pub mod unpack;

use crate::mv::disassembler::Encode;
use libra::file_format::*;
use std::fmt::Write;
use anyhow::Error;
use itertools::Itertools;
use crate::mv::disassembler::code::exp::operators::{BinaryOp, Abort, Not};
use crate::mv::disassembler::code::exp::ret::Ret;
use crate::mv::disassembler::code::exp::cast::Cast;
use crate::mv::disassembler::code::exp::ld::Ld;
use crate::mv::disassembler::code::exp::function::FnCall;
use crate::mv::disassembler::code::exp::loc::Loc;
use crate::mv::disassembler::code::exp::lt::Let;
use crate::mv::disassembler::code::exp::pack::Pack;
use crate::mv::disassembler::code::exp::unpack::Unpack;
use crate::mv::disassembler::code::exp::rf::{FieldRef, Ref, Deref, WriteRef};
use crate::mv::disassembler::code::exp::block::Block;

#[derive(Debug)]
pub struct ExpLoc<'a> {
    index: usize,
    exp: Box<Exp<'a>>,
}

impl<'a> ExpLoc<'a> {
    pub fn new(index: usize, val: Exp<'a>) -> ExpLoc<'a> {
        ExpLoc {
            index,
            exp: Box::new(val),
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn range(&self) -> (usize, usize) {
        if let Some((mut l, mut r)) = self.exp.source_range() {
            if self.index < l {
                l = self.index;
            }

            if self.index > r {
                r = self.index;
            }

            (l, r)
        } else {
            (self.index, self.index)
        }
    }

    pub fn val(self) -> Exp<'a> {
        *self.exp
    }
}

impl<'a> SourceRange for ExpLoc<'a> {
    fn source_range(&self) -> Option<(usize, usize)> {
        Some(self.range())
    }
}

impl<'a> SourceRange for &ExpLoc<'a> {
    fn source_range(&self) -> Option<(usize, usize)> {
        Some(self.range())
    }
}

impl<'a> SourceRange for (usize, usize) {
    fn source_range(&self) -> Option<(usize, usize)> {
        Some((self.0, self.1))
    }
}

impl<'a> SourceRange for Option<(usize, usize)> {
    fn source_range(&self) -> Option<(usize, usize)> {
        self.map(|(l, r)| (l, r))
    }
}

pub trait SourceRange {
    fn source_range(&self) -> Option<(usize, usize)>;
}

impl<'a> AsRef<Exp<'a>> for ExpLoc<'a> {
    fn as_ref(&self) -> &Exp<'a> {
        &self.exp
    }
}

impl<'a> AsMut<Exp<'a>> for ExpLoc<'a> {
    fn as_mut(&mut self) -> &mut Exp<'a> {
        self.exp.as_mut()
    }
}

impl<'a> Encode for ExpLoc<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        self.exp.encode(w, indent)
    }
}

#[derive(Debug)]
pub enum Exp<'a> {
    Abort(Abort<'a>),
    Ld(Ld),
    Error(Bytecode),
    Local(Loc<'a>),
    Cast(Cast<'a>),
    BinaryOp(BinaryOp<'a>),
    Basket(ExpLoc<'a>),
    Not(Not<'a>),
    FnCall(FnCall<'a>),
    Let(Let<'a>),
    Pack(Pack<'a>),
    Unpack(Unpack<'a>),
    Ret(Ret<'a>),
    FieldRef(FieldRef<'a>),
    Ref(Ref<'a>),
    Deref(Deref<'a>),
    WriteRef(WriteRef<'a>),
    Loop(Block<'a>),
    While(ExpLoc<'a>, Block<'a>),
    If(ExpLoc<'a>, Block<'a>, Option<Block<'a>>),
    Break,
    Continue,
    Nop,
}

impl<'a> Exp<'a> {
    pub fn is_nop(&self) -> bool {
        match self {
            Exp::Nop => true,
            _ => false,
        }
    }

    pub fn source_range(&self) -> Option<(usize, usize)> {
        match self {
            Exp::Abort(a) => a.source_range(),
            Exp::Error(_) => None,
            Exp::Ld(ld) => ld.source_range(),
            Exp::Local(l) => l.source_range(),
            Exp::Cast(cast) => cast.source_range(),
            Exp::FnCall(f_call) => f_call.source_range(),
            Exp::BinaryOp(exp) => exp.source_range(),
            Exp::Basket(e) => e.source_range(),
            Exp::Not(e) => e.source_range(),
            Exp::Let(lt) => lt.source_range(),
            Exp::Pack(p) => p.source_range(),
            Exp::Unpack(u) => u.source_range(),
            Exp::Ret(r) => r.source_range(),
            Exp::FieldRef(rf) => rf.source_range(),
            Exp::Ref(r) => r.source_range(),
            Exp::Deref(drf) => drf.source_range(),
            Exp::WriteRef(wr) => wr.source_range(),
            Exp::Loop(b) => b.source_range(),
            Exp::While(e, b) => find_range(vec![e.source_range(), b.source_range()]),
            Exp::If(e, t, f) => find_range(vec![
                e.source_range(),
                t.source_range(),
                f.as_ref().and_then(|f| f.source_range()),
            ]),
            Exp::Break => None,
            Exp::Continue => None,
            Exp::Nop => None,
        }
    }
}

impl<'a> Encode for Exp<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        if indent != 0 {
            write!(w, "{s:width$}", s = "", width = indent as usize)?;
        }

        match self {
            Exp::Abort(a) => a.encode(w, indent)?,
            Exp::Local(loc) => loc.encode(w, indent)?,
            Exp::Cast(cast) => cast.encode(w, indent)?,
            Exp::FnCall(call) => call.encode(w, indent)?,
            Exp::Ret(ret) => ret.encode(w, indent)?,
            Exp::Nop => {
                // no-op
            }
            Exp::Error(b) => {
                write!(w, "Err [opcode: {:?}]", b)?;
            }
            Exp::BinaryOp(op) => op.encode(w, indent)?,
            Exp::Basket(inner) => {
                w.write_str("(")?;
                inner.encode(w, indent)?;
                w.write_str(")")?;
            }
            Exp::Not(not) => not.encode(w, indent)?,
            Exp::Let(lt) => lt.encode(w, indent)?,
            Exp::Pack(pack) => pack.encode(w, indent)?,
            Exp::Unpack(unpack) => unpack.encode(w, indent)?,
            Exp::FieldRef(rf) => rf.encode(w, indent)?,
            Exp::Loop(block) => {
                w.write_str("loop ")?;
                block.encode(w, indent)?;
            }
            Exp::If(condition, true_branch, false_branch) => {
                w.write_str("if (")?;
                condition.encode(w, 0)?;
                w.write_str(") ")?;
                true_branch.encode(w, indent)?;
                if let Some(false_branch) = false_branch {
                    w.write_str(" else ")?;
                    false_branch.encode(w, indent)?;
                }
            }
            Exp::Break => {
                w.write_str("break")?;
            }
            Exp::While(condition, body) => {
                w.write_str("while (")?;
                condition.encode(w, 0)?;
                w.write_str(") ")?;
                body.encode(w, indent)?;
            }
            Exp::Ref(rf) => rf.encode(w, indent)?,
            Exp::Deref(drf) => drf.encode(w, indent)?,
            Exp::WriteRef(wr) => wr.encode(w, indent)?,
            Exp::Continue => {
                w.write_str("continue")?;
            }
            Exp::Ld(ld) => ld.encode(w, indent)?,
        }
        Ok(())
    }
}

pub fn find_range<T, S>(range_list: T) -> Option<(usize, usize)>
where
    T: IntoIterator<Item = S>,
    S: SourceRange,
{
    let sorted_index_list = range_list
        .into_iter()
        .map(|p| p.source_range())
        .filter_map(|p| p)
        .flat_map(|p| vec![p.0, p.1])
        .sorted()
        .collect::<Vec<_>>();
    sorted_index_list
        .first()
        .and_then(|f| sorted_index_list.last().map(|l| (*f, *l)))
}
