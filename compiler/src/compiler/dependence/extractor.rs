use anyhow::Result;
use libra::libra_types::language_storage::ModuleId;
use std::path::PathBuf;
use libra::move_lang::{parse_program, errors};
use libra::move_lang::parser::ast::{Definition, ModuleDefinition, Script};
use crate::compiler::builder::convert_path;
use std::collections::HashSet;
use libra::move_core_types::identifier::Identifier;
use libra::libra_types::account_address::AccountAddress;
use libra::move_lang::parser::ast::*;
use libra::libra_vm::CompiledModule;


pub fn extract_from_source(targets: &[PathBuf]) -> Result<HashSet<ModuleId>> {
    let mut extractor = DefinitionUses::default();
    let (files, pprog_and_comments_res) = parse_program(&convert_path(targets)?, &vec![])?;
    match pprog_and_comments_res {
        Ok((program, _)) => {
            for def in program.source_definitions {
                extractor.extract(&def)?;
            }
        }
        Err(errs) => errors::report_errors(files, errs),
    }

    Ok(extractor.imports())
}

pub fn extract_from_bytecode(bytecode: &[u8]) -> Result<HashSet<ModuleId>> {
    let mut extractor = BytecodeUses::default();
    extractor.extract(CompiledModule::deserialize(bytecode)?)?;
    Ok(extractor.imports())
}

#[derive(Default)]
pub struct DefinitionUses {
    imports: HashSet<ModuleId>,
}

impl DefinitionUses {
    pub fn extract(&mut self, def: &Definition) -> Result<()> {
        match def {
            Definition::Module(module) => self.module(module)?,
            Definition::Address(_, _, modules) => {
                for module in modules {
                    self.module(module)?;
                }
            }
            Definition::Script(script) => self.script(script)?,
        }
        Ok(())
    }

    fn module(&mut self, module: &ModuleDefinition) -> Result<()> {
        self.uses(&module.uses)?;

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

        for func in &module.functions {
            self.function(func)?;
        }

        Ok(())
    }

    fn script(&mut self, script: &Script) -> Result<()> {
        self.uses(&script.uses)?;
        self.function(&script.function)
    }

    fn uses(&mut self, uses: &[(ModuleIdent, Option<ModuleName>)]) -> Result<()> {
        for (dep, _) in uses {
            let ident = &dep.0.value;
            let name = Identifier::new(ident.name.0.value.to_owned())?;
            let address = AccountAddress::new(ident.address.clone().to_u8());
            self.imports.insert(ModuleId::new(address, name));
        }
        Ok(())
    }

    fn function(&mut self, func: &Function) -> Result<()> {
        self.signature(&func.signature)?;
        self.internal_usages(&func.body)
    }

    fn signature(&mut self, signature: &FunctionSignature) -> Result<()> {
        for (_, v_type) in &signature.parameters {
            self.type_usages(&v_type.value)?;
        }
        self.type_usages(&signature.return_type.value)
    }

    fn internal_usages(&mut self, func: &FunctionBody) -> Result<()> {
        match &func.value {
            FunctionBody_::Defined((seq, _, exp)) => {
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

    fn type_usages(&mut self, v_type: &Type_) -> Result<()> {
        match v_type {
            Type_::Unit => { /*No-op*/ }
            Type_::Multiple(s_types) => {
                for s_type in s_types {
                    self.s_type_usages(&s_type.value)?;
                }
            }
            Type_::Apply(access, s_types) => {
                for s_type in s_types {
                    self.s_type_usages(&s_type.value)?;
                }
                self.access_usages(&access.value)?;
            }
            Type_::Ref(_, s_type) => {
                self.s_type_usages(&s_type.value)?;
            }
            Type_::Fun(s_types, s_type) => {
                self.s_type_usages(&s_type.value)?;
                for s_type in s_types {
                    self.s_type_usages(&s_type.value)?;
                }
            }
        }
        Ok(())
    }

    fn block_usages(&mut self, seq: &[SequenceItem]) -> Result<()> {
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

    fn bind_usages(&mut self, bind: &Bind_) -> Result<()> {
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

    fn access_usages(&mut self, access: &ModuleAccess_) -> Result<()> {
        match access {
            ModuleAccess_::QualifiedModuleAccess(ident, _name) => {
                let ident = &ident.0.value;
                self.imports.insert(ModuleId::new(
                    AccountAddress::new(ident.address.clone().to_u8()),
                    Identifier::new(ident.name.0.value.to_owned())?,
                ));
            }
            ModuleAccess_::ModuleAccess(_, _)
            | ModuleAccess_::Name(_)
            | ModuleAccess_::Global(_) => { /*no-op*/ }
        }
        Ok(())
    }

    fn s_type_usages(&mut self, s_type: &Type_) -> Result<()> {
        match s_type {
            Type_::Apply(module_access, s_types) => {
                self.access_usages(&module_access.value)?;
                for s_type in s_types {
                    self.s_type_usages(&s_type.value)?;
                }
            }
            Type_::Ref(_, s_type) => {
                self.s_type_usages(&s_type.value)?;
            }
            Type_::Fun(s_types, s_type) => {
                for s_type in s_types {
                    self.s_type_usages(&s_type.value)?;
                }
                self.s_type_usages(&s_type.value)?;
            }
            Type_::Unit => {}
            Type_::Multiple(s_types) => {
                for s_type in s_types {
                    self.s_type_usages(&s_type.value)?;
                }
            }
        }
        Ok(())
    }

    fn expresion_usages(&mut self, exp: &Exp_) -> Result<()> {
        match exp {
            Exp_::Value(_)
            | Exp_::Move(_)
            | Exp_::Copy(_)
            | Exp_::Unit
            | Exp_::Break
            | Exp_::Continue
            | Exp_::Lambda(_, _)
            | Exp_::Spec(_)
            | Exp_::Index(_, _)
            | Exp_::InferredNum(_)
            | Exp_::UnresolvedError => { /*no op*/ }
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
            Exp_::Block((seq, _, exp)) => {
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
            | Exp_::Loop(e)
            | Exp_::UnaryExp(_, e)
            | Exp_::Borrow(_, e)
            | Exp_::Dot(e, _)
            | Exp_::Annotate(e, _) => {
                self.expresion_usages(&e.value)?;
            }
            Exp_::Return(e) => {
                if let Some(e) = e {
                    self.expresion_usages(&e.value)?;
                }
            }
            Exp_::BinopExp(e1, _, e2) => {
                self.expresion_usages(&e1.value)?;
                self.expresion_usages(&e2.value)?;
            }
            Exp_::Name(access, s_types) => {
                self.access_usages(&access.value)?;
                if let Some(s_types) = s_types {
                    for s_type in s_types {
                        self.s_type_usages(&s_type.value)?;
                    }
                }
            }
            Exp_::Cast(e1, s_type) => {
                self.expresion_usages(&e1.value)?;
                self.s_type_usages(&s_type.value)?;
            }
        }
        Ok(())
    }

    pub fn imports(self) -> HashSet<ModuleId> {
        self.imports
    }
}

#[derive(Default)]
pub struct BytecodeUses {
    imports: HashSet<ModuleId>,
}

impl BytecodeUses {
    pub fn imports(self) -> HashSet<ModuleId> {
        self.imports
    }

    pub fn extract(&mut self, module: CompiledModule) -> Result<()> {
        let module = module.into_inner();
        let mut module_handles = module.module_handles;
        if module_handles.len() > 1 {
            // Remove self module with 0 index.
            module_handles.remove(0);
        }

        for module_handle in module_handles {
            let name = module.identifiers[module_handle.name.0 as usize].as_str().to_owned();
            let address = module.address_identifiers[module_handle.address.0 as usize];
            self.imports.insert(ModuleId::new(address, Identifier::new(name)?));
        }

        Ok(())
    }
}