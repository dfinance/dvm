use crate::mv::disassembler::code::exp::{Exp, ExpLoc};
use crate::mv::disassembler::imports::{Imports, Import};
use crate::mv::disassembler::generics::Generic;
use crate::mv::disassembler::types::{extract_type_signature, FType};

use libra::prelude::*;
use libra::bf::*;
use libra::file_format::*;

use crate::mv::disassembler::code::locals::{Locals, Local};
use crate::mv::disassembler::code::iter::BytecodeIterator;
use crate::mv::disassembler::code::exp::operators::{BinaryOp, Op, pop, nop, Abort, Not};
use crate::mv::disassembler::code::exp::ret::Ret;
use crate::mv::disassembler::code::exp::cast::{CastType, Cast};
use crate::mv::disassembler::code::exp::ld::Ld;
use crate::mv::disassembler::code::exp::function::{FnCall, BuildIn};
use crate::mv::disassembler::code::exp::loc::{Loc, LocAccess};
use crate::mv::disassembler::code::exp::lt::Let;
use crate::mv::disassembler::code::exp::rf::{FieldRef, Ref, Deref, WriteRef};
use crate::mv::disassembler::code::exp::pack::{PackField, Pack};
use crate::mv::disassembler::code::exp::unpack::Unpack;
use crate::mv::disassembler::code::exp::branching::{br_true, br_false, br};
use crate::mv::disassembler::unit::UnitAccess;

pub trait Context<'a> {
    fn pop_exp(&mut self) -> ExpLoc<'a>;

    fn last_exp(&self) -> Option<&ExpLoc<'a>>;

    fn pop2_exp(&mut self) -> (ExpLoc<'a>, ExpLoc<'a>);

    fn pop_exp_vec(&mut self, exp_count: usize) -> Vec<ExpLoc<'a>>;

    fn module_import(&self, module: &ModuleHandle) -> Option<Import<'a>>;

    fn extract_signature(&self, type_params: Option<&SignatureIndex>) -> Vec<FType<'a>>;

    fn local_var(&self, index: u8) -> Local<'a>;

    fn opcode_offset(&self) -> usize;

    fn last(&self) -> Option<&ExpLoc<'a>>;

    fn pack_fields(&mut self, def: &StructDefinition) -> Vec<PackField<'a>>;

    fn translate_block(&mut self, block_size: usize) -> Vec<ExpLoc<'a>>;

    fn next_opcode(&mut self) -> Option<&Bytecode>;

    fn loc(&self, exp: Exp<'a>) -> ExpLoc<'a>;

    fn opcode_by_relative_offset(&self, offset: isize) -> &Bytecode;

    fn opcode_by_absolute_offset(&self, offset: usize) -> &Bytecode;

    fn end_offset(&self) -> usize;

    fn remaining_code(&self) -> &[Bytecode];

    fn err(&self) -> Exp<'a>;
}

pub struct Translator<'a, 'b, 'c, A>
where
    A: UnitAccess,
{
    expressions: Vec<ExpLoc<'a>>,
    locals: &'b Locals<'a>,
    unit: &'a A,
    imports: &'a Imports<'a>,
    type_params: &'b [Generic],
    opcode_iter: &'c mut BytecodeIterator<'a>,
    flow_graph: &'c VMControlFlowGraph,
    end_offset: usize,
    ret_len: usize,
}

impl<'a, 'b, 'c, A> Translator<'a, 'b, 'c, A>
where
    A: UnitAccess,
{
    pub fn new(
        opcode_iter: &'c mut BytecodeIterator<'a>,
        ret_len: usize,
        opcodes_count: usize,
        locals: &'b Locals<'a>,
        unit: &'a A,
        imports: &'a Imports<'a>,
        type_params: &'b [Generic],
        flow_graph: &'c VMControlFlowGraph,
    ) -> Translator<'a, 'b, 'c, A> {
        let start_offset = opcode_iter.index();
        Translator {
            opcode_iter,
            expressions: vec![],
            locals,
            unit,
            imports,
            type_params,
            ret_len,
            end_offset: start_offset + opcodes_count,
            flow_graph,
        }
    }

    pub fn translate(&mut self) {
        loop {
            if self.end_offset > self.opcode_iter.index() {
                if let Some(opcode) = self.opcode_iter.next() {
                    let exp = self.next_exp(opcode);
                    self.expressions
                        .push(ExpLoc::new(self.opcode_iter.index(), exp));
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    pub fn next_exp(&mut self, opcode: &Bytecode) -> Exp<'a> {
        match opcode {
            Bytecode::Pop => pop(),
            Bytecode::Not => Not::new(self),
            Bytecode::Abort => Abort::new(self),
            Bytecode::Add => BinaryOp::new(Op::Add, self),
            Bytecode::Sub => BinaryOp::new(Op::Sub, self),
            Bytecode::Mul => BinaryOp::new(Op::Mul, self),
            Bytecode::Mod => BinaryOp::new(Op::Mod, self),
            Bytecode::Div => BinaryOp::new(Op::Div, self),
            Bytecode::BitOr => BinaryOp::new(Op::BitOr, self),
            Bytecode::BitAnd => BinaryOp::new(Op::BitAnd, self),
            Bytecode::Xor => BinaryOp::new(Op::Xor, self),
            Bytecode::Or => BinaryOp::new(Op::Or, self),
            Bytecode::And => BinaryOp::new(Op::And, self),
            Bytecode::Eq => BinaryOp::new(Op::Eq, self),
            Bytecode::Neq => BinaryOp::new(Op::Neq, self),
            Bytecode::Lt => BinaryOp::new(Op::Lt, self),
            Bytecode::Gt => BinaryOp::new(Op::Gt, self),
            Bytecode::Le => BinaryOp::new(Op::Le, self),
            Bytecode::Ge => BinaryOp::new(Op::Ge, self),
            Bytecode::Shl => BinaryOp::new(Op::Shl, self),
            Bytecode::Shr => BinaryOp::new(Op::Shr, self),
            Bytecode::Nop => nop(),
            Bytecode::Ret => Ret::new(self.ret_len, self),
            Bytecode::CastU8 => Cast::new(CastType::U8, self),
            Bytecode::CastU64 => Cast::new(CastType::U64, self),
            Bytecode::CastU128 => Cast::new(CastType::U128, self),
            Bytecode::LdU8(val) => Ld::u8(*val),
            Bytecode::LdU64(val) => Ld::u64(*val),
            Bytecode::LdU128(val) => Ld::u128(*val),
            Bytecode::LdConst(index) => Ld::ld_const(*index, self.unit),
            Bytecode::LdTrue => Ld::bool(true),
            Bytecode::LdFalse => Ld::bool(false),
            Bytecode::Call(index) => FnCall::plain(index, None, self, self.unit),
            Bytecode::CallGeneric(index) => {
                let inst = self.unit.function_instantiation(*index);
                FnCall::plain(&inst.handle, Some(&inst.type_parameters), self, self.unit)
            }
            Bytecode::Exists(index) => {
                FnCall::build_in(BuildIn::Exists, index, None, 1, self, self.unit)
            }
            Bytecode::ExistsGeneric(index) => self.build_in(*index, BuildIn::Exists, 1, opcode),
            Bytecode::MoveFrom(index) => {
                FnCall::build_in(BuildIn::MoveFrom, index, None, 1, self, self.unit)
            }
            Bytecode::MoveFromGeneric(index) => self.build_in(*index, BuildIn::MoveFrom, 1, opcode),
            Bytecode::MoveTo(index) => {
                FnCall::build_in(BuildIn::MoveTo, index, None, 2, self, self.unit)
            }
            Bytecode::MoveToGeneric(index) => self.build_in(*index, BuildIn::MoveTo, 2, opcode),
            Bytecode::ImmBorrowGlobal(index) => {
                FnCall::build_in(BuildIn::BorrowGlobal, index, None, 1, self, self.unit)
            }
            Bytecode::ImmBorrowGlobalGeneric(index) => {
                self.build_in(*index, BuildIn::BorrowGlobal, 1, opcode)
            }
            Bytecode::MutBorrowGlobal(index) => {
                FnCall::build_in(BuildIn::BorrowGlobalMut, index, None, 1, self, self.unit)
            }
            Bytecode::MutBorrowGlobalGeneric(index) => {
                self.build_in(*index, BuildIn::BorrowGlobalMut, 1, opcode)
            }
            Bytecode::CopyLoc(index) => Loc::new(false, LocAccess::Copy, *index, self),
            Bytecode::MoveLoc(index) => Loc::new(false, LocAccess::Move, *index, self),
            Bytecode::StLoc(index) => Let::new(*index, self),
            Bytecode::Pack(index) => Pack::new(index, None, self, self.unit),
            Bytecode::PackGeneric(index) => {
                if let Some(inst) = self.unit.struct_def_instantiation(*index) {
                    Pack::new(&inst.def, Some(&inst.type_parameters), self, self.unit)
                } else {
                    Exp::Error(opcode.clone())
                }
            }
            Bytecode::Unpack(def) => Unpack::new(def, None, self, self.unit),
            Bytecode::UnpackGeneric(index) => {
                if let Some(inst) = self.unit.struct_def_instantiation(*index) {
                    Unpack::new(&inst.def, Some(&inst.type_parameters), self, self.unit)
                } else {
                    Exp::Error(opcode.clone())
                }
            }
            Bytecode::MutBorrowField(index) => FieldRef::new(index, true, self, self.unit),
            Bytecode::MutBorrowFieldGeneric(index) => {
                if let Some(field_index) = self.unit.field_instantiation(*index) {
                    FieldRef::new(&field_index.handle, true, self, self.unit)
                } else {
                    Exp::Error(opcode.clone())
                }
            }
            Bytecode::ImmBorrowField(index) => FieldRef::new(index, false, self, self.unit),
            Bytecode::ImmBorrowFieldGeneric(index) => {
                if let Some(field_index) = self.unit.field_instantiation(*index) {
                    FieldRef::new(&field_index.handle, false, self, self.unit)
                } else {
                    Exp::Error(opcode.clone())
                }
            }
            Bytecode::FreezeRef => self.pop_exp().val(),
            Bytecode::MutBorrowLoc(index) => Ref::new(*index, true, self),
            Bytecode::ImmBorrowLoc(index) => Ref::new(*index, false, self),
            Bytecode::ReadRef => Deref::new(self),
            Bytecode::WriteRef => WriteRef::new(self),

            Bytecode::BrTrue(true_offset) => br_true(*true_offset as usize, self),
            Bytecode::BrFalse(offset) => br_false(*offset as usize, self),
            Bytecode::Branch(offset) => br(*offset as usize, self),
        }
    }

    pub fn expressions(self) -> Vec<ExpLoc<'a>> {
        self.expressions
    }

    fn build_in(
        &mut self,
        index: StructDefInstantiationIndex,
        kind: BuildIn,
        params_count: usize,
        opcode: &Bytecode,
    ) -> Exp<'a> {
        if let Some(def) = self.unit.struct_def_instantiation(index) {
            FnCall::build_in(
                kind,
                &def.def,
                Some(&def.type_parameters),
                params_count,
                self,
                self.unit,
            )
        } else {
            Exp::Error(opcode.clone())
        }
    }

    fn take_by_offset(&mut self, offset: usize) -> Vec<ExpLoc<'a>> {
        let mut buffer = Vec::new();
        loop {
            if let Some(exp) = self.expressions.last() {
                if exp.index() >= offset {
                    if let Some(exp) = self.expressions.pop() {
                        buffer.insert(0, exp);
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        buffer
    }
}

impl<'a, 'b, 'c, A> Context<'a> for Translator<'a, 'b, 'c, A>
where
    A: UnitAccess,
{
    fn pop_exp(&mut self) -> ExpLoc<'a> {
        self.expressions
            .pop()
            .unwrap_or_else(|| ExpLoc::new(0, Exp::Nop))
    }

    fn last_exp(&self) -> Option<&ExpLoc<'a>> {
        self.expressions.last()
    }

    fn pop2_exp(&mut self) -> (ExpLoc<'a>, ExpLoc<'a>) {
        let second = self.pop_exp();
        let first = self.pop_exp();
        (first, second)
    }

    fn pop_exp_vec(&mut self, exp_count: usize) -> Vec<ExpLoc<'a>> {
        let len = self.expressions.len();
        if exp_count > len {
            let mut res = self.expressions.split_off(0);
            for _ in 0..exp_count - len {
                res.push(self.loc(Exp::Nop))
            }
            res
        } else {
            self.expressions
                .split_off(self.expressions.len() - exp_count)
                .into_iter()
                .collect()
        }
    }

    fn module_import(&self, module: &ModuleHandle) -> Option<Import<'a>> {
        let module_name = self.unit.identifier(module.name);
        let module_address = self.unit.address(module.address);
        self.imports.get_import(module_address, module_name)
    }

    fn extract_signature(&self, type_params: Option<&SignatureIndex>) -> Vec<FType<'a>> {
        type_params
            .map(|index| {
                self.unit
                    .signature(*index)
                    .0
                    .iter()
                    .map(|t| extract_type_signature(self.unit, t, self.imports, self.type_params))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![])
    }

    fn local_var(&self, index: u8) -> Local<'a> {
        self.locals.get(index as usize)
    }

    fn opcode_offset(&self) -> usize {
        self.opcode_iter.index()
    }

    fn last(&self) -> Option<&ExpLoc<'a>> {
        self.expressions.last()
    }

    fn pack_fields(&mut self, def: &StructDefinition) -> Vec<PackField<'a>> {
        match &def.field_information {
            StructFieldInformation::Native => vec![],
            StructFieldInformation::Declared(fields) => self
                .pop_exp_vec(fields.len())
                .into_iter()
                .zip(fields)
                .map(|(exp, def)| PackField {
                    name: self.unit.identifier(def.name),
                    value: exp,
                })
                .collect(),
        }
    }

    fn translate_block(&mut self, block_size: usize) -> Vec<ExpLoc<'a>> {
        let mut translator = Translator::new(
            self.opcode_iter,
            self.ret_len,
            block_size,
            self.locals,
            self.unit,
            self.imports,
            self.type_params,
            self.flow_graph,
        );
        translator.translate();
        translator.expressions
    }

    fn next_opcode(&mut self) -> Option<&Bytecode> {
        self.opcode_iter.next()
    }

    fn loc(&self, exp: Exp<'a>) -> ExpLoc<'a> {
        ExpLoc::new(self.opcode_offset(), exp)
    }

    fn opcode_by_relative_offset(&self, offset: isize) -> &Bytecode {
        self.opcode_iter.by_relative(offset)
    }

    fn opcode_by_absolute_offset(&self, offset: usize) -> &Bytecode {
        self.opcode_iter.absolute(offset)
    }

    fn end_offset(&self) -> usize {
        self.end_offset
    }

    fn remaining_code(&self) -> &[Bytecode] {
        self.opcode_iter.remaining_code()
    }

    fn err(&self) -> Exp<'a> {
        Exp::Error(self.opcode_iter.by_relative(0).clone())
    }
}
