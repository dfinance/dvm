use std::fmt::Write;
use anyhow::Error;
use serde::{Serialize, Deserialize};
use crate::mv::disassembler::code::exp::{ExpLoc, Exp, find_range, SourceRange};
use crate::mv::disassembler::code::translator::Context;
use crate::mv::disassembler::{Encode, write_array};

/// Return expression.
#[derive(Debug, Serialize, Deserialize)]
pub struct Ret<'a> {
    /// Result tuple.
    #[serde(borrow)]
    pub ret_list: Vec<ExpLoc<'a>>,
    /// is explicit return required.
    pub explicit_keyword: bool,
}

impl<'a> Ret<'a> {
    /// Create a new `Ret` expression.
    pub fn exp(ret_len: usize, ctx: &mut impl Context<'a>) -> Exp<'a> {
        let params = (0..ret_len).map(|_| ctx.pop_exp()).collect::<Vec<_>>();
        Exp::Ret(Ret {
            ret_list: params.into_iter().rev().collect(),
            explicit_keyword: false,
        })
    }

    /// Returns `true` if the function empty tuple.
    pub fn is_empty(&self) -> bool {
        self.ret_list.is_empty()
    }

    /// Returns `true` if the explicit return keyword required.
    pub fn is_explicit(&self) -> bool {
        self.explicit_keyword
    }
}

impl<'a> SourceRange for Ret<'a> {
    fn source_range(&self) -> Option<(usize, usize)> {
        find_range(&self.ret_list)
    }
}

impl<'a> Encode for Ret<'a> {
    fn encode<W: Write>(&self, w: &mut W, _: usize) -> Result<(), Error> {
        if self.explicit_keyword {
            w.write_str("return ")?;
        }

        match self.ret_list.len() {
            0 => {
                //no-op
            }
            1 => {
                self.ret_list[0].encode(w, 0)?;
            }
            _ => {
                write_array(w, "(", ", ", &self.ret_list, ")")?;
            }
        }
        Ok(())
    }
}
