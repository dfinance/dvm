use libra::libra_vm::file_format::*;
use crate::mv::disassembler::imports::{Imports, Import};
use crate::mv::disassembler::functions::Param;
use crate::mv::disassembler::{Encode, write_array};
use anyhow::Error;
use serde::export::fmt::Write;
use crate::mv::disassembler::types::{FType, extract_type_signature};
use crate::mv::disassembler::generics::Generic;
use std::sync::atomic::{Ordering, AtomicBool};
use std::rc::Rc;
use crate::mv::disassembler::code::translator::Translator;
use crate::mv::disassembler::code::exp::Exp;
use crate::mv::disassembler::code::locals::{Locals, Local};

pub struct Body<'a> {
    middle: Vec<Exp<'a>>,
    locals: Locals<'a>,
}

impl<'a> Body<'a> {
    pub fn new<'b>(
        code: &'a CodeUnit,
        ret_len: usize,
        module: &'a CompiledModuleMut,
        params: &'b [Param<'a>],
        imports: &'a Imports,
        type_params: &'b [Generic],
    ) -> Body<'a> {
        println!("bytecode {:?}", code);

        let locals = Locals::new(
            params,
            module,
            imports,
            type_params,
            &module.signatures[code.locals.0 as usize],
        );
        println!("locals {:?}", locals);

        let mut iter = code.code.iter();
        let mut translator = Translator::new(
            &mut iter,
            ret_len,
            code.code.len(),
            &locals,
            module,
            imports,
            type_params,
        );
        translator.translate();

        Body {
            middle: dbg!(translator.expressions()),
            locals,
        }
    }
}

impl<'a> Encode for Body<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        for local in &self.locals.inner {
            match local {
                Local::Var(var) => {
                    writeln!(w)?;
                    write!(w, "{s:width$}let ", s = "", width = indent as usize)?;
                    var.encode(w, 0)?;
                    w.write_str(";")?;
                }
                Local::Param(_) => {
                    //no-op
                }
            }
        }

        if !self.locals.inner.is_empty() {
            writeln!(w)?;
        }

        for (index, middle) in self.middle.iter().enumerate() {
            if middle.is_none() {
                continue;
            }

            writeln!(w)?;
            middle.encode(w, indent)?;

            if !(middle.is_ret() || middle.is_abort()) {
                w.write_str(";")?;
            }
        }
        Ok(())
    }
}
