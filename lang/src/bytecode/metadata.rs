use compiler::disassembler::unit::CompiledUnit as Unit;
use compiler::disassembler::Config;
use compiler::disassembler::Disassembler;
use compiler::disassembler::unit::SourceUnit;
use compiler::disassembler::types::FType;
use compiler::disassembler::functions::{FunctionsDef, Param};
use compiler::disassembler::structs::{StructDef, Field as FieldAst};
use compiler::disassembler::types::FullStructName;
use compiler::disassembler::generics::Generic;
use anyhow::{Result, Error};
use libra::file_format::{SignatureToken, Kind};
use compiler::disassembler::unit::UnitAccess;
use std::convert::TryFrom;
use compiler::disassembler::Encode;
use std::fmt::Write;

/// Function metadata.
#[derive(Debug, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FunctionMeta {
    /// Function name.
    pub name: String,
    /// Function visibility.
    pub is_public: bool,
    /// Is function native.
    pub is_native: bool,
    /// Function type parameters.
    pub type_params: Vec<String>,
    /// Function arguments.
    pub arguments: Vec<String>,
    /// Function return types.
    pub ret: Vec<String>,
}

/// Struct metadata.
#[derive(Debug, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StructMeta {
    /// Struct name.
    pub name: String,
    /// Is struct resource.
    pub is_resource: bool,
    /// Struct type parameters.
    pub type_params: Vec<String>,
    /// Struct fields.
    pub fields: Vec<FieldMeta>,
}

/// Field metadata.
#[derive(Debug, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FieldMeta {
    /// Field name.
    pub name: String,
    /// Field type.
    pub f_type: String,
}

/// Bytecode metadata.
#[derive(Debug, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Metadata {
    /// Script metadata.
    Script {
        /// Script type parameters.
        type_parameters: Vec<Kind>,
        /// Script arguments.
        arguments: Vec<SignatureToken>,
    },
    /// Module metadata.
    Module {
        /// Module name.
        name: String,
        /// Module functions.
        functions: Vec<FunctionMeta>,
        /// Module structs.
        structs: Vec<StructMeta>,
    },
}

/// Extract bytecode metadata.
pub fn extract_bytecode_metadata(bytecode: &[u8]) -> Result<Metadata> {
    let unit = Unit::new(bytecode)?;
    let disasm = Disassembler::new(
        &unit,
        Config {
            light_version: true,
        },
    );
    let ast = disasm.make_source_unit();

    Ok(match ast {
        SourceUnit::Script(_) => {
            if let Some((_, type_parameters, params)) = unit.script_info() {
                Metadata::Script {
                    type_parameters: type_parameters.to_vec(),
                    arguments: unit.signature(params).0.to_vec(),
                }
            } else {
                Metadata::Script {
                    type_parameters: vec![],
                    arguments: vec![],
                }
            }
        }
        SourceUnit::Module(module) => {
            let functions = module
                .functions()
                .iter()
                .map(FunctionMeta::try_from)
                .collect::<Result<_>>()?;
            let structs = module
                .structs()
                .iter()
                .map(StructMeta::try_from)
                .collect::<Result<_>>()?;

            Metadata::Module {
                name: module.name().to_string(),
                functions,
                structs,
            }
        }
    })
}

impl TryFrom<&FunctionsDef<'_>> for FunctionMeta {
    type Error = Error;

    fn try_from(def: &FunctionsDef<'_>) -> Result<Self, Self::Error> {
        Ok(FunctionMeta {
            name: def.name().to_owned(),
            is_public: def.is_public(),
            is_native: def.is_native(),
            type_params: render(def.type_params())?,
            arguments: render(def.params())?,
            ret: render(def.ret())?,
        })
    }
}

fn render<I, R>(param: I) -> Result<Vec<String>, Error>
where
    I: IntoIterator<Item = R>,
    R: MetadataRender,
{
    param
        .into_iter()
        .map(|i| MetadataRender::render(&i))
        .collect()
}

impl TryFrom<&StructDef<'_>> for StructMeta {
    type Error = Error;

    fn try_from(def: &StructDef<'_>) -> Result<Self, Self::Error> {
        let fields = def
            .fields()
            .iter()
            .filter(|f| f.name() != "dummy_field")
            .map(|f| {
                Ok(FieldMeta {
                    name: f.name().to_owned(),
                    f_type: f.f_type().render()?,
                })
            })
            .collect::<Result<_>>()?;

        Ok(StructMeta {
            name: def.name().to_owned(),
            is_resource: def.is_nominal_resource(),
            type_params: render(def.type_params())?,
            fields,
        })
    }
}

trait MetadataRender {
    fn render(&self) -> Result<String, Error>;
}

impl MetadataRender for &Generic {
    fn render(&self) -> Result<String, Error> {
        let mut buf = String::new();
        self.as_name().encode(&mut buf, 0)?;
        Ok(buf)
    }
}

impl MetadataRender for &FType<'_> {
    fn render(&self) -> Result<String, Error> {
        let mut w = String::new();

        match self {
            FType::Primitive(name) => {
                w.write_str(name)?;
            }
            FType::Generic(type_param) => {
                type_param.as_name().encode(&mut w, 0)?;
            }
            FType::Ref(t) => {
                w.write_str("&")?;
                t.encode(&mut w, 0)?;
            }
            FType::RefMut(t) => {
                w.write_str("&mut ")?;
                t.encode(&mut w, 0)?;
            }
            FType::Vec(t) => {
                w.write_str("vector<")?;
                t.encode(&mut w, 0)?;
                w.write_str(">")?;
            }
            FType::Struct(name) => {
                w.write_str(&name.render()?)?;
            }
            FType::StructInst(name, generics) => {
                w.write_str(&name.render()?)?;

                if !generics.is_empty() {
                    write!(&mut w, "<")?;
                    for (index, generic) in generics.iter().enumerate() {
                        generic.encode(&mut w, 0)?;
                        if index != generics.len() - 1 {
                            w.write_str(", ")?;
                        }
                    }
                    write!(w, ">")?;
                }
            }
        }
        Ok(w)
    }
}

impl MetadataRender for &FullStructName<'_> {
    fn render(&self) -> Result<String, Error> {
        let mut w = String::new();

        if let Some(import) = self.import() {
            let address = import
                .address()
                .as_ref()
                .iter()
                .copied()
                .skip_while(|d| *d == 0u8)
                .collect::<Vec<u8>>();
            w.push_str("0x");
            w.push_str(&hex::encode(address));
            w.push_str("::");
            import.encode(&mut w, 0)?;
            w.push_str("::");
        }
        w.push_str(self.name());

        Ok(w)
    }
}

impl MetadataRender for &Param<'_> {
    fn render(&self) -> Result<String, Error> {
        self.f_type().as_ref().render()
    }
}

impl TryFrom<&FieldAst<'_>> for FieldMeta {
    type Error = Error;

    fn try_from(field: &FieldAst<'_>) -> Result<Self, Self::Error> {
        Ok(FieldMeta {
            name: field.name().to_owned(),
            f_type: field.f_type().render()?,
        })
    }
}
