#[allow(dead_code)]
/// Block of expressions in curly braces.
pub mod block;
/// Branching algorithms.
pub mod branching;
/// Cast.
pub mod cast;
/// Function call.
pub mod function;
/// Load literal or constant.
pub mod ld;
/// Load local variable.
pub mod loc;
/// Local variable assignment.
pub mod lt;
/// Build in operators.
pub mod operators;
/// Struct constructor.
pub mod pack;
/// Return statement.
pub mod ret;
/// Reference.
pub mod rf;
/// Struct destructor.
pub mod unpack;

use std::fmt::Write;
use libra::file_format::*;
use anyhow::Error;
use itertools::Itertools;
use serde::{Serialize, Deserialize};
use crate::mv::disassembler::Encode;
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

/// Expression wrapper that adds bytecode location of this expression.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExpLoc<'a> {
    index: usize,
    #[serde(borrow)]
    exp: Box<Exp<'a>>,
}

impl<'a> ExpLoc<'a> {
    /// Create a new `ExpLoc`.
    pub fn new(index: usize, val: Exp<'a>) -> ExpLoc<'a> {
        ExpLoc {
            index,
            exp: Box::new(val),
        }
    }

    /// Returns expression start index in the bytecode.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns index range of the expression.
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

    /// Returns inner expression.
    pub fn val(self) -> Exp<'a> {
        *self.exp
    }
}

impl<'a> Default for ExpLoc<'a> {
    fn default() -> Self {
        ExpLoc::new(0, Exp::Nop)
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

/// Range in the bytecode.
pub trait SourceRange {
    /// Returns index range.
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

/// Move expression.
#[derive(Debug, Serialize, Deserialize)]
pub enum Exp<'a> {
    /// Abort. (abort)
    #[serde(borrow)]
    Abort(Abort<'a>),
    /// Load literal or constant. (5)
    Ld(Ld),
    /// Disassembler error.
    #[serde(skip)]
    // TODO: serde remote type impl
    Error(Bytecode),
    /// Local variable.
    #[serde(borrow)]
    Local(Loc<'a>),
    /// Cast types. (as)
    #[serde(borrow)]
    Cast(Cast<'a>),
    /// Binary operation.
    #[serde(borrow)]
    BinaryOp(BinaryOp<'a>),
    /// Expression in parentheses.
    #[serde(borrow)]
    Basket(ExpLoc<'a>),
    /// Logical negation.
    #[serde(borrow)]
    Not(Not<'a>),
    /// Function call.
    #[serde(borrow)]
    FnCall(FnCall<'a>),
    /// Local variable assignment.
    #[serde(borrow)]
    Let(Let<'a>),
    /// Struct constructor.
    #[serde(borrow)]
    Pack(Pack<'a>),
    /// Struct destructor.
    #[serde(borrow)]
    Unpack(Unpack<'a>),
    /// Return.
    #[serde(borrow)]
    Ret(Ret<'a>),
    /// Structures field access.
    #[serde(borrow)]
    FieldRef(FieldRef<'a>),
    /// Reference.
    #[serde(borrow)]
    Ref(Ref<'a>),
    /// Dereference.
    #[serde(borrow)]
    Deref(Deref<'a>),
    /// Assign reference.
    #[serde(borrow)]
    WriteRef(WriteRef<'a>),
    /// Infinite Loop.
    #[allow(dead_code)]
    #[serde(borrow)]
    Loop(Block<'a>),
    /// While loop.
    // #[serde(borrow = "'a + 'a")]
    While(#[serde(borrow)] ExpLoc<'a>, #[serde(borrow)] Block<'a>),
    /// If else expression.
    If(
        #[serde(borrow)] ExpLoc<'a>,
        #[serde(borrow)] Block<'a>,
        #[serde(borrow)] Option<Block<'a>>,
    ),
    /// Break.
    Break,
    /// Continue.
    Continue,
    /// Nothing.
    Nop,
}

impl<'a> Exp<'a> {
    /// Returns `true` if the current expression is `Exp::Nop`.
    pub fn is_nop(&self) -> bool {
        match self {
            Exp::Nop => true,
            _ => false,
        }
    }

    /// Returns `true` if the current expression is `Exp::Ret`.
    pub fn ret(&mut self) -> Option<&mut Ret<'a>> {
        match self {
            Exp::Ret(ret) => Some(ret),
            _ => None,
        }
    }

    /// Returns bytecode range of the curent expression.
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

/// Returns bytecode range of the given expressions.
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
