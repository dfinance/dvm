use crate::mv::disassembler::types::FType;
use crate::mv::disassembler::{Encode, write_array};
use libra::libra_vm::file_format::*;
use libra::libra_types::account_address::AccountAddress;

use std::fmt::Write;
use anyhow::Error;
use crate::mv::disassembler::code::locals::Local;
use crate::mv::disassembler::imports::Import;

#[derive(Debug)]
pub enum Exp<'a> {
    Abort(Box<Exp<'a>>),
    LdU64(u64),
    LdU8(u8),
    LdU128(u128),
    LdBool(bool),
    Error(Bytecode),
    Local(Local<'a>),
    Cast(Box<Exp<'a>>, &'static str),
    Call(FunctionCall<'a>),
    BinaryOp(Box<Exp<'a>>, &'static str, Box<Exp<'a>>),
    Basket(Box<Exp<'a>>),
    Not(Box<Exp<'a>>),
    Const(Const),
    BuildInFunction(BuildInFunctionCall<'a>),
    GetTxnSenderAddress,
    Let(Local<'a>, Box<Exp<'a>>),
    Pack(Pack<'a>),
    Unpack(Unpack<'a>),
    Ret(Vec<Exp<'a>>),
    Ref(bool, &'a str, Box<Exp<'a>>),
    Nop,
}

impl<'a> Exp<'a> {
    pub fn is_ret(&self) -> bool {
        match self {
            Exp::Ret(_) => true,
            _ => false,
        }
    }

    pub fn is_abort(&self) -> bool {
        match self {
            Exp::Abort(_) => true,
            _ => false,
        }
    }

    pub fn is_none(&self) -> bool {
        match self {
            Exp::Nop => true,
            _ => false,
        }
    }

    pub fn wrap_binary_op(self) -> Exp<'a> {
        match &self {
            Exp::BinaryOp(_, _, _) => Exp::Basket(Box::new(self)),
            _ => self,
        }
    }
}

impl<'a> Encode for Exp<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        if indent != 0 {
            write!(w, "{s:width$}", s = "", width = indent as usize)?;
        }

        match self {
            Exp::Abort(exp) => {
                w.write_str("abort ")?;
                exp.encode(w, 0)?;
            }
            Exp::Local(l) => {
                l.write_name(w)?;
            }
            Exp::Cast(src, dst) => {
                w.write_str("(")?;
                src.encode(w, 0)?;
                write!(w, " as {})", dst)?;
            }
            Exp::LdU8(val) => {
                write!(w, "{}u8", val)?;
            }
            Exp::LdU64(val) => {
                write!(w, "{}", val)?;
            }
            Exp::LdU128(val) => {
                write!(w, "{}u128", val)?;
            }
            Exp::LdBool(val) => {
                write!(w, "{}", val)?;
            }
            Exp::Call(call) => {
                call.encode(w, 0)?;
            }
            Exp::Ret(exp) => {
                match exp.len() {
                    0 => {
                        //no-op
                    },
                    1 => {
                        exp[0].encode(w, 0)?;
                    },
                    _ => {
                        write_array(w, "(", ", ", exp, ")")?;
                    }
                }
            }
            Exp::Nop => {
                // no-op
            }
            Exp::Error(b) => {
                write!(w, "Err [opcode: {:?}]", b)?;
            }
            Exp::BinaryOp(left, op, right) => {
                left.encode(w, 0)?;
                write!(w, " {} ", op)?;
                right.encode(w, 0)?;
            }
            Exp::Basket(inner) => {
                w.write_str("(")?;
                inner.encode(w, 0)?;
                w.write_str(")")?;
            }
            Exp::Not(inner) => {
                w.write_str("!")?;
                inner.encode(w, 0)?;
            }
            Exp::Const(cnst) => {
                cnst.encode(w, 0)?;
            }
            Exp::GetTxnSenderAddress => {
                w.write_str("0x1::Transaction::sender()")?;
            }
            Exp::BuildInFunction(call) => {
                call.encode(w, 0)?;
            }
            Exp::Let(local, exp) => {
                local.write_name(w)?;
                if !exp.is_none() {
                    w.write_str(" = ")?;
                    exp.encode(w, 0)?;
                }
            }
            Exp::Pack(pack) => {
                pack.encode(w, 0)?;
            }
            Exp::Unpack(unpack) => {
                unpack.encode(w, 0)?;
            }
            Exp::Ref(is_mut, name, instance) => {
                w.write_str("&")?;
                if *is_mut {
                    w.write_str("mut ")?;
                }
                instance.encode(w, 0)?;
                w.write_str(".")?;
                w.write_str(name)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct BuildInFunctionCall<'a> {
    pub name: &'a str,
    pub type_param_name: StructName<'a>,
    pub type_params: Vec<FType<'a>>,
    pub params: Vec<Exp<'a>>,
}

impl<'a> Encode for BuildInFunctionCall<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        w.write_str(self.name)?;
        w.write_str("<")?;
        self.type_param_name.encode(w, 0)?;
        if !self.type_params.is_empty() {
            write_array(w, "<", ", ", &self.type_params, ">")?;
        }
        w.write_str(">")?;
        write_array(w, "(", ", ", &self.params, ")")
    }
}

#[derive(Debug)]
pub struct FunctionCall<'a> {
    pub module: Option<Import<'a>>,
    pub name: &'a str,
    pub type_params: Vec<FType<'a>>,
    pub params: Vec<Exp<'a>>,
}

impl<'a> Encode for FunctionCall<'a> {
    fn encode<W: Write>(&self, w: &mut W, _: u8) -> Result<(), Error> {
        if let Some(import) = &self.module {
            import.encode(w, 0)?;
            w.write_str("::")?;
        }
        write!(w, "{}", self.name)?;
        if !self.type_params.is_empty() {
            write_array(w, "<", ", ", &self.type_params, ">")?;
        }
        write_array(w, "(", ", ", &self.params, ")")
    }
}

#[derive(Debug)]
pub struct StructName<'a> {
    pub name: &'a str,
    pub import: Option<Import<'a>>,
}

impl<'a> Encode for StructName<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        if let Some(import) = &self.import {
            import.encode(w, indent)?;
        }
        w.write_str(self.name)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum Const {
    Address(AccountAddress),
    Vector(Vec<u8>),
}

impl Encode for Const {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        match self {
            Const::Address(addr) => {
                write!(w, "0x{}", addr)?;
            }
            Const::Vector(vec) => {
                write!(w, "x\"{}\"", hex::encode(&vec))?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Pack<'a> {
    pub module: Option<Import<'a>>,
    pub name: &'a str,
    pub type_params: Vec<FType<'a>>,
    pub fields: Vec<PackField<'a>>,
}

impl<'a> Encode for Pack<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        if let Some(module) = &self.module {
            module.encode(w, 0)?;
            w.write_str("::")?;
        }
        w.write_str(self.name)?;
        if !self.type_params.is_empty() {
            write_array(w, "<", ", ", &self.type_params, ">")?;
        }

        write_array(w, " { ", ", ", &self.fields, " }")
    }
}

#[derive(Debug)]
pub struct PackField<'a> {
    pub name: &'a str,
    pub value: Exp<'a>,
}

impl<'a> Encode for PackField<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        w.write_str(self.name)?;
        w.write_str(": ")?;

        if self.value.is_none() {
            w.write_str("_")?;
        } else {
            self.value.encode(w, 0)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct UnpackProto<'a> {
    pub module: Option<Import<'a>>,
    pub name: &'a str,
    pub type_params: Vec<FType<'a>>,
    pub fields: Vec<&'a str>,
    pub source: Box<Exp<'a>>,
}

#[derive(Debug)]
pub struct Unpack<'a> {
    pub module: Option<Import<'a>>,
    pub name: &'a str,
    pub type_params: Vec<FType<'a>>,
    pub fields: Vec<PackField<'a>>,
    pub source: Box<Exp<'a>>,
}

impl<'a> Encode for Unpack<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        if let Some(module) = &self.module {
            module.encode(w, 0)?;
            w.write_str("::")?;
        }
        w.write_str(self.name)?;
        if !self.type_params.is_empty() {
            write_array(w, "<", ", ", &self.type_params, ">")?;
        }

        write_array(w, " { ", ", ", &self.fields, " }")?;
        w.write_str(" = ")?;
        self.source.encode(w, 0)
    }
}
