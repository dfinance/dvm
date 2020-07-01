use crate::mv::disassembler::code::exp::{
    Exp, Const, StructName, FunctionCall, BuildInFunctionCall, Pack, PackField, Unpack, UnpackProto,
};
use crate::mv::disassembler::imports::{Imports, Import};
use crate::mv::disassembler::generics::Generic;
use crate::mv::disassembler::types::{extract_type_signature, FType};
use libra::libra_vm::file_format::*;
use libra::move_core_types::value::MoveValue;
use libra::libra_types::account_address::AccountAddress;
use crate::mv::disassembler::code::locals::Locals;
use std::slice::Iter;

pub struct Translator<'a, 'b, 'c> {
    expressions: Vec<Exp<'a>>,
    locals: &'b Locals<'a>,
    module: &'a CompiledModuleMut,
    imports: &'a Imports<'a>,
    type_params: &'b [Generic],
    opcode_iter: &'c mut Iter<'b, Bytecode>,
    opcodes_count: usize,
    ret_len: usize,
}

impl<'a, 'b, 'c> Translator<'a, 'b, 'c> {
    pub fn new(
        opcode_iter: &'c mut Iter<'b, Bytecode>,
        ret_len: usize,
        opcodes_count: usize,
        locals: &'b Locals<'a>,
        module: &'a CompiledModuleMut,
        imports: &'a Imports<'a>,
        type_params: &'b [Generic],
    ) -> Translator<'a, 'b, 'c> {
        Translator {
            opcode_iter,
            expressions: vec![],
            locals,
            module,
            imports,
            type_params,
            opcodes_count,
            ret_len,
        }
    }

    pub fn translate(&mut self) {
        for _ in 0..self.opcodes_count {
            if let Some(opcode) = self.opcode_iter.next() {
                let exp_box = self.next_exp(opcode);
                let forwards_exp = exp_box.get_forwards_exp().map(|forwards_exp| {
                    let mut translator = Translator::new(
                        self.opcode_iter,
                        self.ret_len,
                        forwards_exp,
                        self.locals,
                        self.module,
                        self.imports,
                        self.type_params,
                    );
                    translator.translate();
                    let mut expressions = translator.expressions.into_iter().rev().collect::<Vec<_>>();

                    for _ in 0..forwards_exp - expressions.len() {
                        expressions.push(Exp::Nop);
                    }

                    expressions
                });

                self.expressions.push(exp_box.finalize(forwards_exp));
            } else {
                break;
            }
        }
    }

    pub fn next_exp(&mut self, opcode: &Bytecode) -> ExpProto<'a> {
        match opcode {
            Bytecode::Pop => ExpProto::Final(Exp::Nop),
            Bytecode::Ret => {
                let params = (0..self.ret_len).map(|_| self.pop()).collect::<Vec<_>>();
                ExpProto::Final(Exp::Ret(params.into_iter().rev().collect()))
            }

            /// todo
            Bytecode::BrTrue(_) => ExpProto::Final(Exp::Error(opcode.clone())),
            Bytecode::BrFalse(_) => ExpProto::Final(Exp::Error(opcode.clone())),
            Bytecode::Branch(_) => ExpProto::Final(Exp::Error(opcode.clone())),

            Bytecode::CastU8 => ExpProto::Final(Exp::Cast(Box::new(self.pop()), "u8")),
            Bytecode::CastU64 => ExpProto::Final(Exp::Cast(Box::new(self.pop()), "u64")),
            Bytecode::CastU128 => ExpProto::Final(Exp::Cast(Box::new(self.pop()), "u128")),
            Bytecode::StLoc(index) => {
                let exp = match self.last() {
                    Exp::Let(_, _) => Exp::Nop,
                    _ => self.pop()
                };
                let local = self.locals.get(*index as usize);
                ExpProto::Final(Exp::Let(local, Box::new(exp)))
            }
            Bytecode::Call(index) => self.make_function_call(index, None),
            Bytecode::CallGeneric(index) => {
                let inst = &self.module.function_instantiations[index.0 as usize];
                self.make_function_call(&inst.handle, Some(&inst.type_parameters))
            }
            Bytecode::Pack(index) => self.pack(index, None),
            Bytecode::PackGeneric(index) => {
                let inst = &self.module.struct_def_instantiations[index.0 as usize];
                self.pack(&inst.def, Some(&inst.type_parameters))
            }
            Bytecode::Unpack(index) => self.unpack(index, None),
            Bytecode::UnpackGeneric(index) => {
                let inst = &self.module.struct_def_instantiations[index.0 as usize];
                self.unpack(&inst.def, Some(&inst.type_parameters))
            }
            /// todo
            Bytecode::ReadRef => ExpProto::Final(Exp::Error(opcode.clone())),
            /// todo
            Bytecode::WriteRef => ExpProto::Final(Exp::Error(opcode.clone())),

            //TODO
            Bytecode::MutBorrowLoc(index) => {
                self.locals.get(*index as usize);
                ExpProto::Final(Exp::Error(opcode.clone()))
            }
            /// todo
            Bytecode::ImmBorrowLoc(index) => {
                self.locals.get(*index as usize);
                ExpProto::Final(Exp::Error(opcode.clone()))
            }

            Bytecode::FreezeRef => ExpProto::Final(self.pop()),
            Bytecode::MutBorrowField(index) => {
                self.field_ref(index, true)
            }
            Bytecode::MutBorrowFieldGeneric(index) => {
                let field_index = &self.module.field_instantiations[index.0 as usize];
                self.field_ref(&field_index.handle, true)
            }
            Bytecode::ImmBorrowField(index) => {
                self.field_ref(index, false)
            }
            Bytecode::ImmBorrowFieldGeneric(index) => {
                let field_index = &self.module.field_instantiations[index.0 as usize];
                self.field_ref(&field_index.handle, false)
            }
            Bytecode::MutBorrowGlobal(index) => {
                self.build_in_function_call("borrow_global_mut", index, None, 1)
            }
            Bytecode::MutBorrowGlobalGeneric(index) => {
                let def = &self.module.struct_def_instantiations[index.0 as usize];
                self.build_in_function_call(
                    "borrow_global_mut",
                    &def.def,
                    Some(&def.type_parameters),
                    1,
                )
            }
            Bytecode::ImmBorrowGlobal(index) => {
                self.build_in_function_call("borrow_global", index, None, 1)
            }
            Bytecode::ImmBorrowGlobalGeneric(index) => {
                let def = &self.module.struct_def_instantiations[index.0 as usize];
                self.build_in_function_call(
                    "borrow_global",
                    &def.def,
                    Some(&def.type_parameters),
                    1,
                )
            }
            Bytecode::Add => self.binary_op(opcode, "+"),
            Bytecode::Sub => self.binary_op(opcode, "-"),
            Bytecode::Mul => self.binary_op(opcode, "*"),
            Bytecode::Mod => self.binary_op(opcode, "%"),
            Bytecode::Div => self.binary_op(opcode, "/"),
            Bytecode::BitOr => self.binary_op(opcode, "|"),
            Bytecode::BitAnd => self.binary_op(opcode, "&"),
            Bytecode::Xor => self.binary_op(opcode, "^"),
            Bytecode::Or => self.binary_op(opcode, "||"),
            Bytecode::And => self.binary_op(opcode, "&&"),
            Bytecode::Not => {
                if let Some(exp) = self.expressions.pop() {
                    ExpProto::Final(Exp::Not(Box::new(exp)))
                } else {
                    ExpProto::Final(Exp::Error(opcode.clone()))
                }
            }
            Bytecode::Eq => self.binary_op(opcode, "=="),
            Bytecode::Neq => self.binary_op(opcode, "!="),
            Bytecode::Lt => self.binary_op(opcode, "<"),
            Bytecode::Gt => self.binary_op(opcode, ">"),
            Bytecode::Le => self.binary_op(opcode, "<="),
            Bytecode::Ge => self.binary_op(opcode, ">="),
            Bytecode::Abort => ExpProto::Final(Exp::Abort(Box::new(self.pop()))),
            Bytecode::GetTxnSenderAddress => ExpProto::Final(Exp::GetTxnSenderAddress),
            Bytecode::Exists(index) => self.build_in_function_call("exists", index, None, 1),
            Bytecode::ExistsGeneric(index) => {
                let def = &self.module.struct_def_instantiations[index.0 as usize];
                self.build_in_function_call("exists", &def.def, Some(&def.type_parameters), 1)
            }
            Bytecode::MoveFrom(index) => self.build_in_function_call("move_from", index, None, 1),
            Bytecode::MoveFromGeneric(index) => {
                let def = &self.module.struct_def_instantiations[index.0 as usize];
                self.build_in_function_call("move_from", &def.def, Some(&def.type_parameters), 1)
            }
            Bytecode::MoveToSender(index) => {
                self.build_in_function_call("move_to_sender", index, None, 1)
            }
            Bytecode::MoveToSenderGeneric(index) => {
                let def = &self.module.struct_def_instantiations[index.0 as usize];
                self.build_in_function_call(
                    "move_to_sender",
                    &def.def,
                    Some(&def.type_parameters),
                    1,
                )
            }
            Bytecode::MoveTo(index) => self.build_in_function_call("move_to", index, None, 2),
            Bytecode::MoveToGeneric(index) => {
                let def = &self.module.struct_def_instantiations[index.0 as usize];
                self.build_in_function_call("move_to", &def.def, Some(&def.type_parameters), 2)
            }
            Bytecode::Shl => self.binary_op(opcode, "<<"),
            Bytecode::Shr => self.binary_op(opcode, ">>"),
            Bytecode::Nop => ExpProto::Final(Exp::Nop),
            Bytecode::CopyLoc(index) | Bytecode::MoveLoc(index) => {
                let local = self.locals.get(*index as usize);
                local.mark_as_used();
                ExpProto::Final(Exp::Local(local))
            }
            Bytecode::LdU8(val) => ExpProto::Final(Exp::LdU8(*val)),
            Bytecode::LdU64(val) => ExpProto::Final(Exp::LdU64(*val)),
            Bytecode::LdU128(val) => ExpProto::Final(Exp::LdU128(*val)),
            Bytecode::LdConst(index) => {
                let constant = &self.module.constant_pool[index.0 as usize];
                if let Some(constant) = constant.deserialize_constant() {
                    match constant {
                        MoveValue::Address(addr) => {
                            ExpProto::Final(Exp::Const(Const::Address(addr)))
                        }
                        MoveValue::Vector(v) => {
                            let val = v
                                .iter()
                                .map(|v| match v {
                                    MoveValue::U8(v) => Some(*v),
                                    _ => None,
                                })
                                .collect::<Option<Vec<u8>>>();
                            if let Some(val) = val {
                                ExpProto::Final(Exp::Const(Const::Vector(val)))
                            } else {
                                ExpProto::Final(Exp::Error(opcode.clone()))
                            }
                        }
                        _ => ExpProto::Final(Exp::Error(opcode.clone())),
                    }
                } else {
                    ExpProto::Final(Exp::Error(opcode.clone()))
                }
            }
            Bytecode::LdTrue => ExpProto::Final(Exp::LdBool(true)),
            Bytecode::LdFalse => ExpProto::Final(Exp::LdBool(false)),
        }
    }

    pub fn expressions(self) -> Vec<Exp<'a>> {
        self.expressions
    }

    fn pop(&mut self) -> Exp<'a> {
        self.expressions.pop().unwrap_or_else(|| Exp::Nop)
    }

    fn last(&self) -> &Exp<'a> {
        self.expressions.last().unwrap_or_else(|| &Exp::Nop)
    }

    fn pop2(&mut self) -> (Exp<'a>, Exp<'a>) {
        let second = self.pop();
        let first = self.pop();
        (first, second)
    }

    fn pop_vec(&mut self, count: usize) -> Vec<Exp<'a>> {
        let len = self.expressions.len();
        if count > len {
            let mut res = self.expressions.split_off(0);
            for _ in 0..count - len {
                res.push(Exp::Nop)
            }
            res
        } else {
            self.expressions.split_off(self.expressions.len() - count)
        }
    }

    fn binary_op(&mut self, opcode: &Bytecode, sign: &'static str) -> ExpProto<'a> {
        let (left, right) = self.pop2();
        ExpProto::Final(Exp::BinaryOp(
            Box::new(left.wrap_binary_op()),
            sign,
            Box::new(right.wrap_binary_op()),
        ))
    }

    fn module_import(&self, module: &ModuleHandle) -> Option<Import<'a>> {
        let module_name = self.module.identifiers[module.name.0 as usize].as_str();
        let module_address = &self.module.address_identifiers[module.address.0 as usize];
        self.imports.get_import(module_address, module_name)
    }

    fn pack_fields(&mut self, def: &StructDefinition) -> Vec<PackField<'a>> {
        match &def.field_information {
            StructFieldInformation::Native => vec![],
            StructFieldInformation::Declared(fields) => {
                self.pop_vec(fields.len())
                    .into_iter()
                    .zip(fields)
                    .map(|(exp, def)| PackField {
                        name: self.module.identifiers[def.name.0 as usize].as_str(),
                        value: exp,
                    })
                    .collect()
            }
        }
    }

    fn field_ref(&mut self, index: &FieldHandleIndex, is_mut: bool) -> ExpProto<'a> {
        let field = &self.module.field_handles[index.0 as usize];
        let def = &self.module.struct_defs[field.owner.0 as usize];

        match &def.field_information {
            StructFieldInformation::Declared(fields) => {
                if let Some(field) = fields.get(field.field as usize) {
                    let name = self.module.identifiers[field.name.0 as usize].as_str();
                    ExpProto::Final(Exp::Ref(is_mut, name, Box::new(self.pop())))
                } else {
                    ExpProto::Final(Exp::Nop)
                }
            }
            StructFieldInformation::Native => {
                ExpProto::Final(Exp::Nop)
            }
        }
    }

    fn pack(
        &mut self,
        index: &StructDefinitionIndex,
        type_params: Option<&SignatureIndex>,
    ) -> ExpProto<'a> {
        let def = &self.module.struct_defs[index.0 as usize];
        let struct_handler = &self.module.struct_handles[def.struct_handle.0 as usize];
        let module = &self.module.module_handles[struct_handler.module.0 as usize];

        let name = self.module.identifiers[struct_handler.name.0 as usize].as_str();

        let fields = self.pack_fields(&def);
        let type_params = self.extract_signature(type_params);

        ExpProto::Final(Exp::Pack(Pack {
            module: self.module_import(module),
            name,
            type_params,
            fields,
        }))
    }

    fn unpack(
        &mut self,
        index: &StructDefinitionIndex,
        type_params: Option<&SignatureIndex>,
    ) -> ExpProto<'a> {
        let def = &self.module.struct_defs[index.0 as usize];
        let struct_handler = &self.module.struct_handles[def.struct_handle.0 as usize];
        let module = &self.module.module_handles[struct_handler.module.0 as usize];

        let name = self.module.identifiers[struct_handler.name.0 as usize].as_str();

        let type_params = self.extract_signature(type_params);

        let fields = match &def.field_information {
            StructFieldInformation::Native => vec![],
            StructFieldInformation::Declared(fields) => fields
                .iter()
                .map(|f| self.module.identifiers[f.name.0 as usize].as_str())
                .collect(),
        };

        let forwards_exp = fields.len();

        ExpProto::Unpack(
            UnpackProto {
                module: self.module_import(module),
                name,
                type_params,
                fields,
                source: Box::new(self.pop()),
            },
            forwards_exp,
        )
    }

    fn build_in_function_call(
        &mut self,
        name: &'static str,
        index: &StructDefinitionIndex,
        type_params: Option<&SignatureIndex>,
        params_count: usize,
    ) -> ExpProto<'a> {
        let def = &self.module.struct_defs[index.0 as usize];
        let struct_handler = &self.module.struct_handles[def.struct_handle.0 as usize];
        let module = &self.module.module_handles[struct_handler.module.0 as usize];

        let import = self.module_import(module);
        let params = self.pop_vec(params_count);

        let type_params = self.extract_signature(type_params);

        ExpProto::Final(Exp::BuildInFunction(BuildInFunctionCall {
            name,
            type_param_name: StructName {
                name: self.module.identifiers[struct_handler.name.0 as usize].as_str(),
                import,
            },
            type_params,
            params,
        }))
    }

    fn make_function_call(
        &mut self,
        f_index: &FunctionHandleIndex,
        type_params: Option<&SignatureIndex>,
    ) -> ExpProto<'a> {
        let handler = &self.module.function_handles[f_index.0 as usize];
        let module = &self.module.module_handles[handler.module.0 as usize];
        let f_name = self.module.identifiers[handler.name.0 as usize].as_str();
        let params_count = self.module.signatures[handler.parameters.0 as usize].len();

        let params = self.pop_vec(params_count);

        let type_params = self.extract_signature(type_params);
        let import = self.module_import(module);

        ExpProto::Final(Exp::Call(FunctionCall {
            module: import,
            name: f_name,
            type_params,
            params,
        }))
    }

    pub fn extract_signature(&self, type_params: Option<&SignatureIndex>) -> Vec<FType<'a>> {
        type_params
            .map(|index| {
                self.module.signatures[index.0 as usize]
                    .0
                    .iter()
                    .map(|t| extract_type_signature(self.module, t, self.imports, self.type_params))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![])
    }
}

pub enum ExpProto<'a> {
    Final(Exp<'a>),
    Unpack(UnpackProto<'a>, usize),
}

impl<'a> ExpProto<'a> {
    pub fn get_forwards_exp(&self) -> Option<usize> {
        match self {
            ExpProto::Final(_) => None,
            ExpProto::Unpack(_, forwards_exp) => Some(*forwards_exp),
        }
    }

    pub fn finalize(self, forwards_exp: Option<Vec<Exp<'a>>>) -> Exp<'a> {
        match self {
            ExpProto::Final(exp) => exp,
            ExpProto::Unpack(unpack, _) => {
                let fields = unpack
                    .fields
                    .into_iter()
                    .zip(forwards_exp.unwrap_or_else(|| vec![]))
                    .map(|(name, exp)| PackField { name, value: exp })
                    .collect();

                Exp::Unpack(Unpack {
                    module: unpack.module,
                    name: unpack.name,
                    type_params: unpack.type_params,
                    fields,
                    source: unpack.source,
                })
            }
        }
    }
}
