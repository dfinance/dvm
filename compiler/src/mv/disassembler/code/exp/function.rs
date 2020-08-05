use crate::mv::disassembler::code::exp::{Exp, ExpLoc, SourceRange, find_range};
use crate::mv::disassembler::code::translator::Context;
use crate::mv::disassembler::types::FType;
use crate::mv::disassembler::imports::Import;
use libra::file_format::{FunctionHandleIndex, SignatureIndex, StructDefinitionIndex};
use crate::mv::disassembler::{Encode, write_array};
use anyhow::Error;
use std::fmt::Write;
use crate::embedded::Bytecode;
use crate::mv::disassembler::unit::UnitAccess;

#[derive(Debug)]
pub enum FnCall<'a> {
    BuildIn {
        kind: BuildIn,
        type_param_name: StructName<'a>,
        type_params: Vec<FType<'a>>,
        params: Vec<ExpLoc<'a>>,
    },
    Plain {
        module: Option<Import<'a>>,
        name: &'a str,
        type_params: Vec<FType<'a>>,
        params: Vec<ExpLoc<'a>>,
    },
}

impl<'a> FnCall<'a> {
    pub fn plain(
        f_index: &FunctionHandleIndex,
        type_params: Option<&SignatureIndex>,
        ctx: &mut impl Context<'a>,
        unit: &'a impl UnitAccess,
    ) -> Exp<'a> {
        let handler = unit.function_handle(*f_index);
        let f_name = unit.identifier(handler.name);

        let params_count = unit.signature(handler.parameters).len();
        let params = ctx.pop_exp_vec(params_count);

        let type_params = ctx.extract_signature(type_params);

        let module_handle = unit.module_handle(handler.module);
        let import = ctx.module_import(module_handle);

        Exp::FnCall(FnCall::Plain {
            module: import,
            name: f_name,
            type_params,
            params,
        })
    }

    pub fn build_in(
        kind: BuildIn,
        index: &StructDefinitionIndex,
        type_params: Option<&SignatureIndex>,
        params_count: usize,
        ctx: &mut impl Context<'a>,
        unit: &'a impl UnitAccess,
    ) -> Exp<'a> {
        if let Some(def) = unit.struct_def(*index) {
            let struct_handler = unit.struct_handle(def.struct_handle);
            let module_handle = unit.module_handle(struct_handler.module);

            let import = ctx.module_import(module_handle);
            let params = ctx.pop_exp_vec(params_count);

            let type_params = ctx.extract_signature(type_params);

            Exp::FnCall(FnCall::BuildIn {
                kind,
                type_param_name: StructName {
                    name: unit.identifier(struct_handler.name),
                    import,
                },
                type_params,
                params,
            })
        } else {
            Exp::Error(kind.bytecode(*index))
        }
    }
}

impl<'a> SourceRange for FnCall<'a> {
    fn source_range(&self) -> Option<(usize, usize)> {
        match self {
            FnCall::BuildIn {
                kind: _,
                type_param_name: _,
                type_params: _,
                params,
            }
            | FnCall::Plain {
                module: _,
                name: _,
                type_params: _,
                params,
            } => find_range(params.iter()),
        }
    }
}

impl<'a> Encode for FnCall<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        match self {
            FnCall::BuildIn {
                kind,
                type_param_name,
                type_params,
                params,
            } => {
                kind.encode(w, indent)?;
                w.write_str("<")?;
                type_param_name.encode(w, 0)?;
                if !type_params.is_empty() {
                    write_array(w, "<", ", ", type_params, ">")?;
                }
                w.write_str(">")?;
                write_array(w, "(", ", ", params, ")")
            }
            FnCall::Plain {
                module,
                name,
                type_params,
                params,
            } => {
                if let Some(import) = module {
                    import.encode(w, 0)?;
                    w.write_str("::")?;
                }
                write!(w, "{}", name)?;
                if !type_params.is_empty() {
                    write_array(w, "<", ", ", type_params, ">")?;
                }
                write_array(w, "(", ", ", params, ")")
            }
        }
    }
}

#[derive(Debug)]
pub enum BuildIn {
    Exists,
    MoveFrom,
    MoveTo,
    BorrowGlobal,
    BorrowGlobalMut,
}

impl BuildIn {
    pub fn bytecode(&self, index: StructDefinitionIndex) -> Bytecode {
        match self {
            BuildIn::Exists => Bytecode::Exists(index),
            BuildIn::MoveFrom => Bytecode::MoveFrom(index),
            BuildIn::MoveTo => Bytecode::MoveTo(index),
            BuildIn::BorrowGlobal => Bytecode::ImmBorrowGlobal(index),
            BuildIn::BorrowGlobalMut => Bytecode::MutBorrowGlobal(index),
        }
    }
}

impl Encode for BuildIn {
    fn encode<W: Write>(&self, w: &mut W, _: usize) -> Result<(), Error> {
        w.write_str(match self {
            BuildIn::Exists => "exists",
            BuildIn::MoveFrom => "move_from",
            BuildIn::MoveTo => "move_to",
            BuildIn::BorrowGlobal => "borrow_global",
            BuildIn::BorrowGlobalMut => "borrow_global_mut",
        })?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct StructName<'a> {
    pub name: &'a str,
    pub import: Option<Import<'a>>,
}

impl<'a> Encode for StructName<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        if let Some(import) = &self.import {
            import.encode(w, indent)?;
        }
        w.write_str(self.name)?;
        Ok(())
    }
}
