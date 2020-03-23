use std::collections::HashSet;
use libra::libra_types::language_storage::ModuleId;
use libra::move_lang::parser::ast::*;
use anyhow::Error;
use libra::libra_types::identifier::Identifier;
use libra::libra_types::account_address::AccountAddress;

#[derive(Default)]
pub struct ImportsExtractor {
    imports: HashSet<ModuleId>,
}

impl ImportsExtractor {
    pub fn extract(&mut self, file_definition: &FileDefinition) -> Result<(), Error> {
        match file_definition {
            FileDefinition::Modules(deps) => {
                for dep in deps {
                    match dep {
                        ModuleOrAddress::Module(module) => {
                            self.usages(&module.uses)?;
                            for func in &module.functions {
                                self.internal_usages(&func.body)?;
                            }
                            for st in &module.structs {
                                match &st.fields {
                                    StructFields::Defined(types) => {
                                        for (_, t) in types {
                                            self.s_type_usages(&t.value)?;
                                        }
                                    }
                                    StructFields::Native(_) => {
                                        //No-op
                                    }
                                }
                            }
                        }
                        ModuleOrAddress::Address(_, _) => {}
                    }
                }
            }
            FileDefinition::Main(main) => {
                self.usages(&main.uses)?;
                self.internal_usages(&main.function.body)?;
            }
        }
        Ok(())
    }

    fn usages(&mut self, deps: &Vec<(ModuleIdent, Option<ModuleName>)>) -> Result<(), Error> {
        for (dep, _) in deps {
            let ident = &dep.0.value;
            let name = Identifier::new(ident.name.0.value.to_owned())?;
            let address = AccountAddress::new(ident.address.clone().to_u8());
            self.imports.insert(ModuleId::new(address, name));
        }
        Ok(())
    }

    fn internal_usages(&mut self, func: &FunctionBody) -> Result<(), Error> {
        match &func.value {
            FunctionBody_::Defined((seq, exp)) => {
                self.block_usages(seq)?;
                if let Some(exp) = exp.as_ref() {
                    self.expresion_usages(&exp.value)?;
                }
            }
            FunctionBody_::Native => {
                // No-op
            }
        }
        Ok(())
    }

    fn type_usages(&mut self, v_type: &Type_) -> Result<(), Error> {
        match v_type {
            Type_::Unit => { /*No-op*/ }
            Type_::Single(s_type) => {
                self.s_type_usages(&s_type.value)?;
            }
            Type_::Multiple(s_types) => {
                for s_type in s_types {
                    self.s_type_usages(&s_type.value)?;
                }
            }
        }
        Ok(())
    }

    fn block_usages(&mut self, seq: &Vec<SequenceItem>) -> Result<(), Error> {
        for item in seq {
            match &item.value {
                SequenceItem_::Seq(exp) => self.expresion_usages(&exp.value)?,
                SequenceItem_::Declare(bind_list, s_type) => {
                    for bind in &bind_list.value {
                        self.bind_usages(&bind.value)?;
                    }
                    if let Some(s_type) = s_type {
                        self.type_usages(&s_type.value)?;
                    }
                }
                SequenceItem_::Bind(bind_list, s_type, exp) => {
                    for bind in &bind_list.value {
                        self.bind_usages(&bind.value)?;
                    }

                    if let Some(s_type) = s_type {
                        self.type_usages(&s_type.value)?;
                    }

                    self.expresion_usages(&exp.value)?;
                }
            }
        }
        Ok(())
    }

    fn bind_usages(&mut self, bind: &Bind_) -> Result<(), Error> {
        match bind {
            Bind_::Var(_) => { /*no-op*/ }
            Bind_::Unpack(access, s_types, binds) => {
                self.access_usages(&access.value)?;
                if let Some(s_types) = s_types {
                    for s_type in s_types {
                        self.s_type_usages(&s_type.value)?;
                    }
                    for bind in binds {
                        self.bind_usages(&bind.1.value)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn access_usages(&mut self, access: &ModuleAccess_) -> Result<(), Error> {
        match access {
            ModuleAccess_::QualifiedModuleAccess(ident, _name) => {
                let ident = &ident.0.value;
                self.imports.insert(ModuleId::new(
                    AccountAddress::new(ident.address.clone().to_u8()),
                    Identifier::new(ident.name.0.value.to_owned())?,
                ));
            }
            ModuleAccess_::ModuleAccess(_, _) | ModuleAccess_::Name(_) => { /*no-op*/ }
        }
        Ok(())
    }

    fn s_type_usages(&mut self, s_type: &SingleType_) -> Result<(), Error> {
        match s_type {
            SingleType_::Apply(module_access, s_types) => {
                self.access_usages(&module_access.value)?;
                for s_type in s_types {
                    self.s_type_usages(&s_type.value)?;
                }
            }
            SingleType_::Ref(_, s_type) => {
                self.s_type_usages(&s_type.value)?;
            }
        }
        Ok(())
    }

    fn expresion_usages(&mut self, exp: &Exp_) -> Result<(), Error> {
        match exp {
            Exp_::Value(_)
            | Exp_::Move(_)
            | Exp_::Copy(_)
            | Exp_::Name(_)
            | Exp_::Unit
            | Exp_::Break
            | Exp_::Continue
            | Exp_::UnresolvedError => { /*no op*/ }
            Exp_::GlobalCall(_, types, exp_list) => {
                for exp in &exp_list.value {
                    self.expresion_usages(&exp.value)?;
                }
                if let Some(types) = types {
                    for s_type in types {
                        self.s_type_usages(&s_type.value)?;
                    }
                }
            }
            Exp_::Call(access, s_types, exp_list) => {
                self.access_usages(&access.value)?;

                if let Some(s_types) = s_types {
                    for s_type in s_types {
                        self.s_type_usages(&s_type.value)?;
                    }
                }

                for exp in &exp_list.value {
                    self.expresion_usages(&exp.value)?;
                }
            }
            Exp_::Pack(access, s_types, exp_list) => {
                self.access_usages(&access.value)?;

                if let Some(s_types) = s_types {
                    for s_type in s_types {
                        self.s_type_usages(&s_type.value)?;
                    }
                }

                for (_, exp) in exp_list {
                    self.expresion_usages(&exp.value)?;
                }
            }
            Exp_::IfElse(eb, et, ef) => {
                self.expresion_usages(&eb.value)?;
                self.expresion_usages(&et.value)?;
                if let Some(ef) = ef {
                    self.expresion_usages(&ef.value)?;
                }
            }
            Exp_::While(eb, eloop) => {
                self.expresion_usages(&eb.value)?;
                self.expresion_usages(&eloop.value)?;
            }
            Exp_::Block((seq, exp)) => {
                self.block_usages(seq)?;
                if let Some(exp) = exp.as_ref() {
                    self.expresion_usages(&exp.value)?;
                }
            }
            Exp_::ExpList(exp_list) => {
                for exp in exp_list {
                    self.expresion_usages(&exp.value)?;
                }
            }
            Exp_::Assign(a, e) => {
                self.expresion_usages(&a.value)?;
                self.expresion_usages(&e.value)?;
            }
            Exp_::Abort(e)
            | Exp_::Dereference(e)
            | Exp_::Return(e)
            | Exp_::Loop(e)
            | Exp_::UnaryExp(_, e)
            | Exp_::Borrow(_, e)
            | Exp_::Dot(e, _)
            | Exp_::Annotate(e, _) => {
                self.expresion_usages(&e.value)?;
            }
            Exp_::BinopExp(e1, _, e2) => {
                self.expresion_usages(&e1.value)?;
                self.expresion_usages(&e2.value)?;
            }
        }
        Ok(())
    }

    pub fn imports(self) -> HashSet<ModuleId> {
        self.imports
    }
}
