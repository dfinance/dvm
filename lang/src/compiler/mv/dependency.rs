use libra::bytecode_verifier::VerifiedModule;
use libra::move_lang::parser::ast::{FileDefinition, ModuleOrAddress};
use libra::move_lang::parser::ast::*;
use libra::move_lang::shared::{Spanned, Loc, Name, Address};
use codespan::Span;
use libra::libra_types::language_storage::ModuleId;
use libra::vm::file_format::{
    CompiledModuleMut, StructDefinition as StructDef, StructFieldInformation, SignatureToken,
    Kind as FKind,
};
use std::convert::TryFrom;

pub struct Dependency {
    id: ModuleId,
    module: CompiledModuleMut,
}

impl Dependency {
    pub fn new(module: VerifiedModule) -> Dependency {
        let module = module.into_inner();

        Dependency {
            id: module.self_id(),
            module: module.into_inner(),
        }
    }

    fn module_name(&self) -> &str {
        self.id.name().as_str()
    }

    pub fn module_id(&self) -> ModuleId {
        self.id.clone()
    }

    fn structs(&self) -> Vec<StructDefinition> {
        self.module
            .struct_defs
            .iter()
            .map(|def| self.map_struct(def))
            .collect()
    }

    fn map_struct(&self, def: &StructDef) -> StructDefinition {
        let fields = match &def.field_information {
            StructFieldInformation::Native => StructFields::Native(Loc::default()),
            StructFieldInformation::Declared {
                field_count,
                fields: start_index,
            } => {
                let mut fields = Vec::with_capacity(*field_count as usize);
                let start_index = start_index.0;
                for index in start_index..start_index + *field_count {
                    let def = &self.module.field_defs[index as usize];
                    let field_name = self.module.identifiers[def.name.0 as usize]
                        .as_str()
                        .to_owned();
                    let type_sign = &self.module.type_signatures[def.signature.0 as usize];
                    fields.push((Field(spanned(field_name)), self.map_type(&type_sign.0)));
                }

                StructFields::Defined(fields)
            }
        };

        let handler = &self.module.struct_handles[def.struct_handle.0 as usize];
        let name = self.module.identifiers[handler.name.0 as usize]
            .as_str()
            .to_owned();
        let resource = if handler.is_nominal_resource {
            Some(Loc::new("res", Span::new(0, 0)))
        } else {
            None
        };

        StructDefinition {
            resource_opt: resource,
            name: StructName(spanned(name)),
            type_parameters: self.map_formals(&handler.type_formals),
            fields,
        }
    }

    fn map_formals(&self, formals: &[FKind]) -> Vec<(Name, Kind)> {
        formals
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let name = format!("T{}", i);
                let params = match k {
                    FKind::All => Kind_::Unknown,
                    FKind::Resource => Kind_::Resource,
                    FKind::Unrestricted => Kind_::Unrestricted,
                };
                (spanned(name), spanned(params))
            })
            .collect()
    }

    fn map_type(&self, sing: &SignatureToken) -> SingleType {
        spanned(match sing {
            SignatureToken::Bool => self.apple_type("bool", vec![]),
            SignatureToken::U8 => self.apple_type("u8", vec![]),
            SignatureToken::U64 => self.apple_type("u64", vec![]),
            SignatureToken::U128 => self.apple_type("u128", vec![]),
            SignatureToken::ByteArray => self.apple_type("bytearray", vec![]),
            SignatureToken::Address => self.apple_type("address", vec![]),
            SignatureToken::Struct(struct_index, types) => {
                let handler = &self.module.struct_handles[struct_index.0 as usize];
                let name = self.module.identifiers[handler.name.0 as usize].as_str();
                let module = &self.module.module_handles[handler.module.0 as usize];
                let module_name = self.module.identifiers[module.name.0 as usize]
                    .as_str()
                    .to_owned();

                let types = types.iter().map(|s| self.map_type(s)).collect();
                if module_name != self.module_name() {
                    SingleType_::Apply(
                        spanned(ModuleAccess_::ModuleAccess(
                            ModuleName(spanned(module_name)),
                            spanned(name.to_owned()),
                        )),
                        types,
                    )
                } else {
                    SingleType_::Apply(
                        spanned(ModuleAccess_::Name(spanned(name.to_owned()))),
                        types,
                    )
                }
            }
            SignatureToken::Reference(signature) => {
                SingleType_::Ref(false, Box::new(self.map_type(signature.as_ref())))
            }
            SignatureToken::MutableReference(signature) => {
                SingleType_::Ref(true, Box::new(self.map_type(signature.as_ref())))
            }
            SignatureToken::TypeParameter(signature) => SingleType_::Apply(
                spanned(ModuleAccess_::Name(spanned(format!("T{}", *signature)))),
                vec![],
            ),
        })
    }

    fn apple_type(&self, name: &str, types: Vec<SingleType>) -> SingleType_ {
        SingleType_::Apply(
            spanned(ModuleAccess_::Name(spanned(name.to_owned()))),
            types,
        )
    }

    fn functions(&self) -> Vec<Function> {
        self.module
            .function_defs
            .iter()
            .map(|def| {
                let handler = &self.module.function_handles[def.function.0 as usize];
                let name = self.module.identifiers[handler.name.0 as usize]
                    .as_str()
                    .to_owned();
                let sign = &self.module.function_signatures[handler.signature.0 as usize];

                let visibility = if def.is_public() {
                    FunctionVisibility::Public(Loc::default())
                } else {
                    FunctionVisibility::Internal
                };

                let parameters = sign
                    .arg_types
                    .iter()
                    .enumerate()
                    .map(|(i, t)| (Var(spanned(format!("arg_{}", i))), self.map_type(t)))
                    .collect();

                let return_type = match sign.return_types.len() {
                    0 => Type_::Unit,
                    1 => Type_::Single(self.map_type(&sign.return_types[0])),
                    _ => Type_::Multiple(
                        sign.return_types.iter().map(|t| self.map_type(t)).collect(),
                    ),
                };

                Function {
                    visibility,
                    signature: FunctionSignature {
                        type_parameters: self.map_formals(&sign.type_formals),
                        parameters,
                        return_type: spanned(return_type),
                    },
                    acquires: vec![],
                    name: FunctionName(spanned(name)),
                    body: spanned(FunctionBody_::Native),
                }
            })
            .collect()
    }

    fn imports(&self) -> Vec<(ModuleIdent, Option<ModuleName>)> {
        self.module
            .module_handles
            .iter()
            .map(|h| {
                (
                    self.module.identifiers[h.name.0 as usize]
                        .as_str()
                        .to_owned(),
                    self.module.address_pool[h.address.0 as usize].to_owned(),
                )
            })
            .filter(|(name, _)| self.module_name() != name)
            .map(|(name, addr)| {
                ModuleIdent(spanned(ModuleIdent_ {
                    name: ModuleName(spanned(name)),
                    address: Address::try_from(addr.as_ref()).unwrap(),
                }))
            })
            .map(|ident| (ident, None))
            .collect()
    }
}

impl Into<FileDefinition> for Dependency {
    fn into(self) -> FileDefinition {
        let def = ModuleDefinition {
            uses: self.imports(),
            name: ModuleName(spanned(self.module_name().to_owned())),
            structs: self.structs(),
            functions: self.functions(),
        };

        FileDefinition::Modules(vec![
            ModuleOrAddress::Address(
                Loc::new("source", Span::new(0, 0)),
                Address::try_from(self.module_id().address().as_ref()).unwrap(),
            ),
            ModuleOrAddress::Module(def),
        ])
    }
}

fn spanned<T>(t: T) -> Spanned<T> {
    Spanned::new(Loc::new("source", Span::new(0, 0)), t)
}
