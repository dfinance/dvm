use libra::file_format::*;
use libra::prelude::*;
use anyhow::Error;
use crate::mv::disassembler::{Encode};
use crate::mv::disassembler::imports::Imports;
use crate::mv::disassembler::generics::Generics;
use crate::mv::disassembler::script::Script as ScriptAst;
use crate::mv::disassembler::module::Module as ModuleAst;
use std::fmt::{Write, Debug};

#[derive(Debug)]
pub enum CompiledUnit {
    Script(CompiledScript),
    Module(CompiledModule),
}

impl CompiledUnit {
    pub fn new(bytecode: &[u8]) -> Result<CompiledUnit, Error> {
        CompiledScript::deserialize(bytecode)
            .map_err(|err| err.finish(Location::Undefined).into_vm_status().into())
            .and_then(|s| {
                if CompiledUnit::is_script(&s) {
                    Ok(CompiledUnit::Script(s))
                } else {
                    CompiledUnit::load_as_module(bytecode)
                }
            })
            .or_else(|_| CompiledUnit::load_as_module(bytecode))
    }

    fn is_script(s: &CompiledScript) -> bool {
        !s.as_inner().code.code.is_empty()
    }

    fn load_as_module(bytecode: &[u8]) -> Result<CompiledUnit, Error> {
        Ok(CompiledUnit::Module(
            CompiledModule::deserialize(bytecode)
                .map_err(|err| err.finish(Location::Undefined).into_vm_status())?,
        ))
    }
}

pub struct Disassembler<'a> {
    unit: &'a CompiledUnit,
    imports: Imports<'a>,
    generics: Generics,
}

impl<'a> Disassembler<'a> {
    pub fn new(unit: &'a CompiledUnit) -> Disassembler<'a> {
        let imports = Imports::new(unit);
        let generics = Generics::new(unit);

        Disassembler {
            unit,
            imports,
            generics,
        }
    }

    pub fn as_source_unit(&'a self) -> SourceUnit<'a> {
        if self.unit.is_script() {
            SourceUnit::Script(ScriptAst::new(self.unit, &self.imports, &self.generics))
        } else {
            SourceUnit::Module(ModuleAst::new(self.unit, &self.imports, &self.generics))
        }
    }
}

pub trait UnitAccess: Debug {
    fn is_script(&self) -> bool;

    fn script_info(&self) -> Option<(&CodeUnit, &Vec<Kind>, SignatureIndex)>;

    fn self_id(&self) -> ModuleId;

    fn module_handles(&self) -> &[ModuleHandle];

    fn module_handle(&self, idx: ModuleHandleIndex) -> &ModuleHandle;

    fn identifiers(&self) -> &[Identifier];

    fn identifier(&self, index: IdentifierIndex) -> &str;

    fn address(&self, index: AddressIdentifierIndex) -> &AccountAddress;

    fn self_module_handle_idx(&self) -> Option<ModuleHandleIndex>;

    fn function_defs(&self) -> &[FunctionDefinition];

    fn function_handle(&self, idx: FunctionHandleIndex) -> &FunctionHandle;

    fn function_instantiation(&self, idx: FunctionInstantiationIndex) -> &FunctionInstantiation;

    fn signature(&self, idx: SignatureIndex) -> &Signature;

    fn struct_defs(&self) -> &[StructDefinition];

    fn struct_def(&self, idx: StructDefinitionIndex) -> Option<&StructDefinition>;

    fn struct_handle(&self, idx: StructHandleIndex) -> &StructHandle;

    fn struct_def_instantiation(
        &self,
        idx: StructDefInstantiationIndex,
    ) -> Option<&StructDefInstantiation>;

    fn field_instantiation(&self, idx: FieldInstantiationIndex) -> Option<&FieldInstantiation>;

    fn constant(&self, idx: ConstantPoolIndex) -> &Constant;

    fn field_handle(&self, idx: FieldHandleIndex) -> Option<&FieldHandle>;
}

impl UnitAccess for CompiledUnit {
    fn is_script(&self) -> bool {
        match self {
            CompiledUnit::Script(_) => true,
            CompiledUnit::Module(_) => false,
        }
    }

    fn script_info(&self) -> Option<(&CodeUnit, &Vec<Kind>, SignatureIndex)> {
        match self {
            CompiledUnit::Script(s) => Some((
                s.code(),
                &s.as_inner().type_parameters,
                s.as_inner().parameters,
            )),
            CompiledUnit::Module(_) => None,
        }
    }

    fn self_id(&self) -> ModuleId {
        match self {
            CompiledUnit::Script(_) => ModuleId::new(
                CORE_CODE_ADDRESS,
                Identifier::new("<SELF>").expect("Valid name."),
            ),
            CompiledUnit::Module(m) => m.self_id(),
        }
    }

    fn module_handles(&self) -> &[ModuleHandle] {
        match self {
            CompiledUnit::Script(s) => s.module_handles(),
            CompiledUnit::Module(m) => m.module_handles(),
        }
    }

    fn module_handle(&self, idx: ModuleHandleIndex) -> &ModuleHandle {
        match self {
            CompiledUnit::Script(s) => s.module_handle_at(idx),
            CompiledUnit::Module(m) => m.module_handle_at(idx),
        }
    }

    fn identifiers(&self) -> &[Identifier] {
        match self {
            CompiledUnit::Script(s) => s.identifiers(),
            CompiledUnit::Module(m) => m.identifiers(),
        }
    }

    fn identifier(&self, index: IdentifierIndex) -> &str {
        match self {
            CompiledUnit::Script(s) => s.identifier_at(index).as_str(),
            CompiledUnit::Module(m) => m.identifier_at(index).as_str(),
        }
    }

    fn address(&self, index: AddressIdentifierIndex) -> &AccountAddress {
        match self {
            CompiledUnit::Script(s) => s.address_identifier_at(index),
            CompiledUnit::Module(m) => m.address_identifier_at(index),
        }
    }

    fn self_module_handle_idx(&self) -> Option<ModuleHandleIndex> {
        match self {
            CompiledUnit::Script(_) => None,
            CompiledUnit::Module(m) => Some(m.self_handle_idx()),
        }
    }

    fn function_defs(&self) -> &[FunctionDefinition] {
        match self {
            CompiledUnit::Script(_) => &[],
            CompiledUnit::Module(m) => &m.as_inner().function_defs,
        }
    }

    fn function_handle(&self, idx: FunctionHandleIndex) -> &FunctionHandle {
        match self {
            CompiledUnit::Script(s) => s.function_handle_at(idx),
            CompiledUnit::Module(m) => m.function_handle_at(idx),
        }
    }

    fn function_instantiation(&self, idx: FunctionInstantiationIndex) -> &FunctionInstantiation {
        match self {
            CompiledUnit::Script(s) => s.function_instantiation_at(idx),
            CompiledUnit::Module(m) => m.function_instantiation_at(idx),
        }
    }

    fn signature(&self, idx: SignatureIndex) -> &Signature {
        match self {
            CompiledUnit::Script(s) => s.signature_at(idx),
            CompiledUnit::Module(m) => m.signature_at(idx),
        }
    }

    fn struct_defs(&self) -> &[StructDefinition] {
        match self {
            CompiledUnit::Script(_) => &[],
            CompiledUnit::Module(m) => &m.as_inner().struct_defs,
        }
    }

    fn struct_def(&self, idx: StructDefinitionIndex) -> Option<&StructDefinition> {
        match self {
            CompiledUnit::Script(_) => None,
            CompiledUnit::Module(m) => Some(m.struct_def_at(idx)),
        }
    }

    fn struct_handle(&self, idx: StructHandleIndex) -> &StructHandle {
        match self {
            CompiledUnit::Script(s) => s.struct_handle_at(idx),
            CompiledUnit::Module(m) => m.struct_handle_at(idx),
        }
    }

    fn struct_def_instantiation(
        &self,
        idx: StructDefInstantiationIndex,
    ) -> Option<&StructDefInstantiation> {
        match self {
            CompiledUnit::Script(_) => None,
            CompiledUnit::Module(m) => Some(m.struct_instantiation_at(idx)),
        }
    }

    fn field_instantiation(&self, idx: FieldInstantiationIndex) -> Option<&FieldInstantiation> {
        match self {
            CompiledUnit::Script(_) => None,
            CompiledUnit::Module(m) => Some(m.field_instantiation_at(idx)),
        }
    }

    fn constant(&self, idx: ConstantPoolIndex) -> &Constant {
        match self {
            CompiledUnit::Script(s) => &s.constant_at(idx),
            CompiledUnit::Module(m) => &m.constant_at(idx),
        }
    }

    fn field_handle(&self, idx: FieldHandleIndex) -> Option<&FieldHandle> {
        match self {
            CompiledUnit::Script(_) => None,
            CompiledUnit::Module(m) => Some(m.field_handle_at(idx)),
        }
    }
}

pub enum SourceUnit<'a> {
    Script(ScriptAst<'a>),
    Module(ModuleAst<'a>),
}

impl<'a> SourceUnit<'a> {
    pub fn write_code<W: Write>(&self, writer: &mut W) -> Result<(), Error> {
        match self {
            SourceUnit::Script(script) => script.encode(writer, 0),
            SourceUnit::Module(module) => module.encode(writer, 0),
        }
    }

    pub fn code_string(&self) -> Result<String, Error> {
        let mut code = String::new();
        self.write_code(&mut code)?;
        Ok(code)
    }
}
