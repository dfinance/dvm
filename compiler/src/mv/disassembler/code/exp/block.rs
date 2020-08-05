use crate::mv::disassembler::code::exp::{SourceRange, ExpLoc, find_range};
use crate::mv::disassembler::{Encode, INDENT};
use std::fmt::Write;
use anyhow::Error;
use super::Exp;

#[derive(Debug)]
pub struct Block<'a> {
    exp: Vec<ExpLoc<'a>>,
    basket: bool,
}

impl<'a> Block<'a> {
    pub fn new(exp: Vec<ExpLoc<'a>>, basket: bool) -> Block<'a> {
        Block { exp, basket }
    }
}

impl<'a> SourceRange for Block<'a> {
    fn source_range(&self) -> Option<(usize, usize)> {
        find_range(&self.exp)
    }
}

impl<'a> Encode for Block<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        if self.basket {
            w.write_str("{")?;
        }

        for (index, exp) in self.exp.iter().enumerate() {
            if exp.as_ref().is_nop() {
                continue;
            }

            match exp.as_ref() {
                Exp::Ret(ret) => {
                    if !ret.is_empty() || ret.is_explicit() {
                        writeln!(w)?;
                    }
                }
                _ => {
                    writeln!(w)?;
                }
            }

            exp.encode(w, indent + INDENT)?;

            match exp.as_ref() {
                Exp::Ret(_) | Exp::Abort(_) | Exp::Break => {
                    //no-op
                }
                Exp::Loop(_) | Exp::If(_, _, _) => {
                    if index != self.exp.len() - 1 {
                        match self.exp[index + 1].as_ref() {
                            Exp::Ret(ret) => {
                                if !ret.is_empty() {
                                    w.write_str(";")?
                                }
                            }
                            _ => w.write_str(";")?,
                        }
                    }
                }
                _ => w.write_str(";")?,
            }
        }
        if self.basket {
            write!(w, "\n{s:width$}}}", s = "", width = indent as usize)?;
        }
        Ok(())
    }
}
