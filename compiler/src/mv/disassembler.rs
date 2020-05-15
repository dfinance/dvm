use anyhow::Result;
use libra::libra_vm::CompiledModule;
use std::fmt::Display;
use serde::export::Formatter;
use core::fmt;
use std::collections::BTreeMap;
use libra::libra_types::language_storage::ModuleId;
use libra::libra_vm::file_format::{
    StructFieldInformation, Kind, SignatureToken, StructHandleIndex, CompiledModuleMut, Signature,
};
use libra::libra_types::account_address::AccountAddress;

const PHANTOM_RESOURCE_NAME: &str = "X_phantom_resource_X_";
const GENERIC_PREFIX: &str = "__G_";

pub struct Config<'a> {
    phantom_resource_name: &'a str,
    generic_prefix: &'a str,
    only_interface: bool,
}

impl<'a> Config<'a> {
    fn new(
        phantom_resource_name: &'a str,
        generic_template: &'a str,
        only_interface: bool,
    ) -> Self {
        Self {
            phantom_resource_name,
            generic_prefix: generic_template,
            only_interface,
        }
    }
}

impl<'a> Default for Config<'a> {
    fn default() -> Self {
        Config::new(PHANTOM_RESOURCE_NAME, GENERIC_PREFIX, false)
    }
}

pub fn module_signature(bytecode: &[u8]) -> Result<ModuleSignature> {
    module_signature_with_configuration(bytecode, Default::default())
}

pub fn module_signature_with_configuration(
    bytecode: &[u8],
    config: Config,
) -> Result<ModuleSignature> {
    let module = CompiledModule::deserialize(&bytecode)?;
    let id = module.self_id();

    let mut imports = Imports::new();
    let functions = extract_functions(&module.as_inner(), &config, &mut imports);

    let mut structs = extract_structs(&module.as_inner(), &config, &mut imports);
    if !config.only_interface
        && functions.has_acursors()
        && !structs.contains(config.phantom_resource_name)
    {
        structs.structs.push(Struct {
            is_nominal_resource: true,
            is_native: false,
            name: config.phantom_resource_name.to_owned(),
            type_params: Default::default(),
            indent_size: 4,
            fields: Params {
                fields: vec![Field {
                    name: "dummy_field".to_string(),
                    f_type: "bool".to_string(),
                }],
                indent_size: 8,
                is_struct_field: true,
            },
        });
    }

    Ok(ModuleSignature {
        id,
        structs,
        functions,
        imports,
    })
}

fn extract_structs(module: &CompiledModuleMut, config: &Config, imports: &mut Imports) -> Structs {
    let structs = module
        .struct_defs
        .iter()
        .map(|def| {
            let handler = &module.struct_handles[def.struct_handle.0 as usize];
            let name = module.identifiers[handler.name.0 as usize].to_string();

            Struct {
                is_nominal_resource: handler.is_nominal_resource,
                is_native: def.field_information == StructFieldInformation::Native,
                name,
                type_params: extract_type_params(&handler.type_parameters, config),
                indent_size: 4,
                fields: Params {
                    fields: extract_fields(module, &def.field_information, config, imports),
                    indent_size: 8,
                    is_struct_field: true,
                },
            }
        })
        .collect();

    Structs { structs }
}

fn extract_type_params(params: &[Kind], config: &Config) -> TypeParams {
    TypeParams {
        params: params
            .iter()
            .enumerate()
            .map(|(i, kind)| TypeParam {
                name: format!("{}{}", config.generic_prefix, i + 1),
                kind: kind.to_owned(),
            })
            .collect(),
    }
}

fn extract_fields(
    module: &CompiledModuleMut,
    info: &StructFieldInformation,
    config: &Config,
    imports: &mut Imports,
) -> Vec<Field> {
    if let StructFieldInformation::Declared(fields) = info {
        fields
            .iter()
            .map(|def| Field {
                name: module.identifiers[def.name.0 as usize].as_str().to_owned(),
                f_type: extract_type_signature(module, &def.signature.0, config, imports),
            })
            .collect()
    } else {
        vec![]
    }
}

fn extract_params(
    module: &CompiledModuleMut,
    info: &Signature,
    config: &Config,
    imports: &mut Imports,
) -> Vec<Field> {
    info.0
        .iter()
        .map(|param| extract_type_signature(module, param, config, imports))
        .enumerate()
        .map(|(i, param)| Field {
            name: format!("_arg_{}", i + 1),
            f_type: param,
        })
        .collect()
}

fn extract_return_value(
    module: &CompiledModuleMut,
    info: &Signature,
    config: &Config,
    imports: &mut Imports,
) -> FuncResult {
    FuncResult {
        ret: info
            .0
            .iter()
            .map(|param| extract_type_signature(module, param, config, imports))
            .collect(),
    }
}

fn extract_type_signature(
    module: &CompiledModuleMut,
    signature: &SignatureToken,
    config: &Config,
    imports: &mut Imports,
) -> String {
    match signature {
        SignatureToken::U8 => "u8".to_owned(),
        SignatureToken::Bool => "bool".to_owned(),
        SignatureToken::U64 => "u64".to_owned(),
        SignatureToken::U128 => "u128".to_owned(),
        SignatureToken::Address => "address".to_owned(),
        SignatureToken::Vector(sign) => format!(
            "vector<{}>",
            extract_type_signature(module, sign.as_ref(), config, imports)
        ),
        SignatureToken::Struct(struct_index) => {
            extract_strict_full_name(module, *struct_index, imports)
        }
        SignatureToken::StructInstantiation(struct_index, typed) => format!(
            "{}<{}>",
            extract_strict_full_name(module, *struct_index, imports),
            typed
                .iter()
                .map(|t| extract_type_signature(module, t, config, imports))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        SignatureToken::Reference(sign) => format!(
            "&{}",
            extract_type_signature(module, sign.as_ref(), config, imports)
        ),
        SignatureToken::MutableReference(sign) => format!(
            "&mut {}",
            extract_type_signature(module, sign.as_ref(), config, imports)
        ),
        SignatureToken::TypeParameter(index) => format!("{}{}", config.generic_prefix, index + 1),
    }
}

fn extract_strict_full_name(
    module: &CompiledModuleMut,
    struct_index: StructHandleIndex,
    imports: &mut Imports,
) -> String {
    let handler = &module.struct_handles[struct_index.0 as usize];
    let type_name = module.identifiers[handler.name.0 as usize].as_str();
    if handler.module.0 == 0 {
        type_name.to_owned()
    } else {
        let module_handler = &module.module_handles[handler.module.0 as usize];
        let module_name = module.identifiers[module_handler.name.0 as usize].as_str();
        let address = &module.address_identifiers[module_handler.address.0 as usize];
        let alias = imports.add(address, module_name);
        format!("{}::{}", alias, type_name)
    }
}

fn extract_functions(
    module: &CompiledModuleMut,
    config: &Config,
    imports: &mut Imports,
) -> Functions {
    let functions = module
        .function_defs
        .iter()
        .map(|def| {
            let handler = &module.function_handles[def.function.0 as usize];
            let name = module.identifiers[handler.name.0 as usize].to_string();
            let signatures = &module.signatures[handler.parameters.0 as usize];

            let (instructions, acquires) = if !def.is_native() {
                let mut body = Vec::new();
                let mut acquires = Vec::new();

                if !config.only_interface {
                    for acquire in &def.acquires_global_resources {
                        let struct_defs = &module.struct_defs[acquire.0 as usize];
                        let handler = &module.struct_handles[struct_defs.struct_handle.0 as usize];
                        let name = module.identifiers[handler.name.0 as usize].to_string();

                        if handler.type_parameters.is_empty() {
                            body.push(Instruction::Borrow(name.to_string()));
                        } else {
                            let params = handler
                                .type_parameters
                                .iter()
                                .map(|param| match param {
                                    Kind::Resource => config.phantom_resource_name.to_string(),
                                    Kind::All | Kind::Copyable => "u64".to_string(),
                                })
                                .collect::<Vec<_>>()
                                .join(", ");
                            body.push(Instruction::Borrow(format!("{}<{}>", name, params)));
                        }

                        acquires.push(name);
                    }

                    body.push(Instruction::Abort(1));
                }
                (body, acquires)
            } else {
                (vec![], vec![])
            };
            Function {
                is_public: def.is_public(),
                is_native: def.is_native(),
                name,
                type_params: extract_type_params(&handler.type_parameters, config),
                params: Params {
                    fields: extract_params(module, &signatures, config, imports),
                    indent_size: 0,
                    is_struct_field: false,
                },
                ret: extract_return_value(
                    module,
                    &module.signatures[handler.return_.0 as usize],
                    config,
                    imports,
                ),
                acquires: Acquires { inner: acquires },
                indent_size: 4,
                body: Block {
                    instructions,
                    indent_size: 4,
                    instructions_indent_size: 8,
                },
            }
        })
        .collect();
    Functions { functions }
}

enum Instruction {
    Abort(u8),
    Borrow(String),
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Instruction::Abort(code) => write!(f, "abort {}", code),
            Instruction::Borrow(resources) => write!(f, "borrow_global<{}>(0x0);", resources),
        }
    }
}

struct Imports {
    uses: BTreeMap<String, BTreeMap<AccountAddress, Option<String>>>,
    indent_size: usize,
}

impl Imports {
    pub fn new() -> Imports {
        Imports {
            uses: Default::default(),
            indent_size: 4,
        }
    }

    pub fn add(&mut self, address: &AccountAddress, name: &str) -> String {
        if let Some(ident) = self.uses.get_mut(name) {
            if let Some(alias) = ident.get(address) {
                if let Some(alias) = alias {
                    alias.to_string()
                } else {
                    name.to_string()
                }
            } else {
                let alias = format!("Other{}{}", name, ident.len());
                ident.insert(*address, Some(alias.clone()));
                alias
            }
        } else {
            let mut alias_map = BTreeMap::new();
            alias_map.insert(address.to_owned(), None);
            self.uses.insert(name.to_owned(), alias_map);
            name.to_owned()
        }
    }
}

impl Display for Imports {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (ident, aliases) in &self.uses {
            for (addr, alias) in aliases {
                if let Some(alias) = alias {
                    writeln!(
                        f,
                        "{:width$}use 0x{address}::{name} as {alias};",
                        "",
                        address = addr,
                        name = ident,
                        width = self.indent_size,
                        alias = alias
                    )?;
                } else {
                    writeln!(
                        f,
                        "{:width$}use 0x{address}::{name};",
                        "",
                        address = addr,
                        name = ident,
                        width = self.indent_size,
                    )?;
                }
            }
        }
        Ok(())
    }
}

struct TypeParam {
    name: String,
    kind: Kind,
}

impl Display for TypeParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.kind {
            Kind::All => write!(f, "{}", self.name),
            Kind::Resource => write!(f, "{}: resource", self.name),
            Kind::Copyable => write!(f, "{}: copyable", self.name),
        }
    }
}

#[derive(Default)]
struct TypeParams {
    params: Vec<TypeParam>,
}

impl Display for TypeParams {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if !self.params.is_empty() {
            write!(
                f,
                "<{}>",
                self.params
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        } else {
            Ok(())
        }
    }
}

struct Field {
    name: String,
    f_type: String,
}

impl Display for Field {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.f_type)
    }
}

#[derive(Default)]
struct Params {
    fields: Vec<Field>,
    indent_size: usize,
    is_struct_field: bool,
}

impl Display for Params {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for (i, field) in self.fields.iter().enumerate() {
            write!(
                f,
                "{s:width$}{field}{end}",
                field = field,
                s = "",
                width = self.indent_size,
                end = if self.is_struct_field {
                    ",\n"
                } else if i == self.fields.len() - 1 {
                    ""
                } else {
                    ", "
                }
            )?;
        }

        Ok(())
    }
}

struct Struct {
    is_nominal_resource: bool,
    is_native: bool,
    name: String,
    type_params: TypeParams,
    indent_size: usize,
    fields: Params,
}

impl Display for Struct {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let nominal_name = if self.is_nominal_resource {
            "resource struct"
        } else if self.is_native {
            "native struct"
        } else {
            "struct"
        };

        if self.is_native {
            writeln!(
                f,
                "{s:width$}{nominal_name} {name}{params};",
                s = "",
                width = self.indent_size,
                nominal_name = nominal_name,
                name = self.name,
                params = self.type_params,
            )
        } else {
            writeln!(
                f,
                "{s:width$}{nominal_name} {name}{params} {{\n{fields}{s:width$}}}",
                s = "",
                width = self.indent_size,
                nominal_name = nominal_name,
                name = self.name,
                params = self.type_params,
                fields = self.fields,
            )
        }
    }
}

struct Structs {
    structs: Vec<Struct>,
}

impl Structs {
    pub fn contains(&self, name: &str) -> bool {
        self.structs.iter().any(|s| s.name == name)
    }
}

impl Display for Structs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for s in &self.structs {
            writeln!(f, "{}", s)?
        }
        Ok(())
    }
}

struct FuncResult {
    ret: Vec<String>,
}

impl Display for FuncResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.ret.len() {
            0 => Ok(()),
            1 => write!(f, ": {}", self.ret[0]),
            _ => write!(f, ": ({})", self.ret.join(", ")),
        }
    }
}

struct Acquires {
    inner: Vec<String>,
}

impl Display for Acquires {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if !self.inner.is_empty() {
            write!(f, " acquires {}", self.inner.join(", "))
        } else {
            Ok(())
        }
    }
}

struct Block {
    instructions: Vec<Instruction>,
    indent_size: usize,
    instructions_indent_size: usize,
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{{")?;
        for i in &self.instructions {
            writeln!(
                f,
                "{s:width$}{i}",
                s = "",
                width = self.instructions_indent_size,
                i = i
            )?;
        }
        writeln!(f, "{s:width$}}}", s = "", width = self.indent_size)?;
        Ok(())
    }
}

struct Function {
    is_public: bool,
    is_native: bool,
    name: String,
    type_params: TypeParams,
    params: Params,
    ret: FuncResult,
    acquires: Acquires,
    indent_size: usize,
    body: Block,
}

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{s:width$}{native}{p}fun {name}{t_params}({params}){return_}{acquires}{native_end}",
            s = "",
            width = self.indent_size,
            p = if self.is_public { "public " } else { "" },
            native = if self.is_native { "native " } else { "" },
            name = self.name,
            t_params = self.type_params,
            params = self.params,
            return_ = self.ret,
            acquires = self.acquires,
            native_end = if self.is_native { ";\n" } else { "" },
        )?;
        if !self.is_native {
            write!(f, " {}", self.body)?;
        }

        Ok(())
    }
}

struct Functions {
    functions: Vec<Function>,
}

impl Functions {
    pub fn has_acursors(&self) -> bool {
        self.functions.iter().any(|f| !f.acquires.inner.is_empty())
    }
}

impl Display for Functions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for s in &self.functions {
            writeln!(f, "{}", s)?
        }
        Ok(())
    }
}

pub struct ModuleSignature {
    id: ModuleId,
    structs: Structs,
    functions: Functions,
    imports: Imports,
}

impl ModuleSignature {
    pub fn self_id(&self) -> &ModuleId {
        &self.id
    }
}

impl Display for ModuleSignature {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "address 0x{address} {{\n\nmodule {name} {{\n{imports}{structs}{functions}}}\n}}",
            address = self.id.address(),
            name = self.id.name(),
            structs = self.structs,
            functions = self.functions,
            imports = self.imports,
        )
    }
}

impl ModuleSignature {}

#[cfg(test)]
mod tests {
    use libra::libra_types::account_address::AccountAddress;
    use ds::MockDataSource;
    use crate::embedded::Compiler;
    use crate::mv::disassembler::module_signature;

    #[test]
    pub fn test_module_signature() {
        let ds = MockDataSource::default();
        let compiler = Compiler::new(ds.clone());
        ds.publish_module(
            compiler
                .compile(
                    include_str!("../../tests/resources/disassembler/base.move"),
                    Some(AccountAddress::new([0x1; 24])),
                )
                .unwrap(),
        )
        .unwrap();
        ds.publish_module(
            compiler
                .compile(
                    include_str!("../../tests/resources/disassembler/base_1.move"),
                    Some(AccountAddress::default()),
                )
                .unwrap(),
        )
        .unwrap();

        for (source, dis) in test_set() {
            let bytecode = compiler
                .compile(source, Some(AccountAddress::default()))
                .unwrap();
            let signature = module_signature(&bytecode).unwrap();
            assert_eq!(&signature.to_string(), dis);

            let bytecode = compiler
                .compile(dis, Some(AccountAddress::default()))
                .unwrap();
            let signature = module_signature(&bytecode).unwrap();
            assert_eq!(&signature.to_string(), dis);
        }
    }

    fn test_set() -> Vec<(&'static str, &'static str)> {
        vec![
            (
                include_str!("../../tests/resources/disassembler/empty_module.move"),
                include_str!("../../tests/resources/disassembler/empty_module_dis.move"),
            ),
            (
                include_str!("../../tests/resources/disassembler/module_with_structs.move"),
                include_str!("../../tests/resources/disassembler/module_with_structs_dis.move"),
            ),
            (
                include_str!("../../tests/resources/disassembler/module_with_functions.move"),
                include_str!("../../tests/resources/disassembler/module_with_functions_dis.move"),
            ),
        ]
    }
}
