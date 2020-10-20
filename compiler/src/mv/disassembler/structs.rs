use crate::mv::disassembler::{Encode, INDENT, Config};
use anyhow::Error;
use std::fmt::Write;
use crate::mv::disassembler::generics::{Generics, Generic, extract_type_params, write_type_parameters};
use libra::file_format::*;
use crate::mv::disassembler::imports::Imports;
use crate::mv::disassembler::types::{FType, extract_type_signature};
use crate::mv::disassembler::unit::{UnitAccess};

/// Struct representation.
pub struct StructDef<'a> {
    is_nominal_resource: bool,
    is_native: bool,
    name: &'a str,
    type_params: Vec<Generic>,
    fields: Vec<Field<'a>>,
}

impl<'a> StructDef<'a> {
    /// Create a new struct.
    pub fn new(
        def: &'a StructDefinition,
        unit: &'a impl UnitAccess,
        generic: &'a Generics,
        imports: &'a Imports<'a>,
        _config: &'a Config,
    ) -> StructDef<'a> {
        let handler = unit.struct_handle(def.struct_handle);
        let name = unit.identifier(handler.name);

        let type_params = extract_type_params(&handler.type_parameters, generic);

        let fields = Self::extract_fields(unit, &def.field_information, imports, &type_params);

        StructDef {
            is_nominal_resource: handler.is_nominal_resource,
            is_native: def.field_information == StructFieldInformation::Native,
            name,
            type_params,
            fields,
        }
    }

    fn extract_fields(
        unit: &'a impl UnitAccess,
        info: &'a StructFieldInformation,
        imports: &'a Imports,
        type_params: &[Generic],
    ) -> Vec<Field<'a>> {
        if let StructFieldInformation::Declared(fields) = info {
            fields
                .iter()
                .map(|def| Field {
                    name: unit.identifier(def.name),
                    f_type: extract_type_signature(unit, &def.signature.0, imports, type_params),
                })
                .collect()
        } else {
            vec![]
        }
    }

    /// Return true if the struct is the nominal resource, false otherwise.
    pub fn is_nominal_resource(&self) -> bool {
        self.is_nominal_resource
    }

    /// Returns struct name.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns struct type parameters.
    pub fn type_params(&self) -> &Vec<Generic> {
        &self.type_params
    }

    /// Returns struct fields.
    pub fn fields(&self) -> &Vec<Field<'a>> {
        &self.fields
    }
}

impl<'a> Encode for StructDef<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        let nominal_name = if self.is_nominal_resource {
            "resource struct"
        } else if self.is_native {
            "native struct"
        } else {
            "struct"
        };

        if self.is_native {
            write!(
                w,
                "{s:width$}{nominal_name} {name}",
                s = "",
                width = indent as usize,
                nominal_name = nominal_name,
                name = self.name,
            )?;
            write_type_parameters(w, &self.type_params)?;
            write!(w, ";")?;
        } else {
            write!(
                w,
                "{s:width$}{nominal_name} {name}",
                s = "",
                width = indent as usize,
                nominal_name = nominal_name,
                name = self.name,
            )?;
            write_type_parameters(w, &self.type_params)?;
            writeln!(w, " {{")?;
            for (index, field) in self.fields.iter().enumerate() {
                field.encode(w, indent + INDENT)?;

                if index != self.fields.len() - 1 {
                    w.write_str(",\n")?;
                } else {
                    w.write_str("\n")?;
                }
            }

            write!(w, "{s:width$}}}", s = "", width = indent as usize,)?;
        }
        Ok(())
    }
}

/// Struct field representation.
pub struct Field<'a> {
    name: &'a str,
    f_type: FType<'a>,
}

impl<'a> Field<'a> {
    /// Returns field name.
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Returns field type.
    pub fn f_type(&self) -> &FType<'a> {
        &self.f_type
    }
}

impl<'a> Encode for Field<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        write!(
            w,
            "{s:width$}{name}: ",
            s = "",
            width = indent as usize,
            name = self.name
        )?;
        self.f_type.encode(w, 0)
    }
}
