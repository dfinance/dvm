use std::rc::Rc;
use std::fmt::Write;
use std::collections::HashSet;
use anyhow::Error;
use libra::file_format::*;
use serde::{Serialize, Deserialize, Deserializer};
use crate::mv::disassembler::{Encode, write_array};
use crate::mv::disassembler::unit::UnitAccess;

const GENERICS_PREFIX: [&str; 22] = [
    "T", "G", "V", "A", "B", "C", "D", "F", "H", "J", "K", "L", "M", "N", "P", "Q", "R", "S", "W",
    "X", "Y", "Z",
];

/// Generics template.
#[derive(Clone, Debug, Serialize)]
// #[serde(transparent)]
pub struct Generics(#[serde(deserialize_with = "Generics::deserialize_rc")] Rc<GenericPrefix>);

/* impl Generics {
    pub fn deserialize_rc<'de, D>(deserializer: D) -> Result<Rc<GenericPrefix>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Rc::new(GenericPrefix::deserialize(deserializer)?))
    }
} */

/// Generics prefix.
#[derive(Debug, Serialize, Deserialize)]
pub enum GenericPrefix {
    /// Simple generic prefix.
    /// Prefix from generic prefix table.
    #[serde(deserialize_with = "deserialize_simple_prefix")]
    SimplePrefix(&'static str),
    /// Random generic prefix.
    Generated(u16),
}

fn deserialize_simple_prefix<'de, D>(deserializer: D) -> Result<&'static str, D::Error>
where
    D: Deserializer<'de>,
{
    let prefix = String::deserialize(deserializer)?;
    let found = GENERICS_PREFIX
        .into_iter()
        .enumerate()
        .find(|(i, item)| *item == &prefix);

    if let Some((index, _)) = found {
        Ok(&GENERICS_PREFIX[index])
    } else {
        Err(serde::de::Error::custom(format!(
            "Unknown Generics Prefix '{}'",
            prefix
        )))
    }
}

impl Generics {
    /// Create a new generics.
    pub fn new(unit: &impl UnitAccess) -> Generics {
        let identifiers: HashSet<&str> = unit.identifiers().iter().map(|i| i.as_str()).collect();

        let generic = if let Some(prefix) = GENERICS_PREFIX
            .iter()
            .find(|prefix| !identifiers.contains(*prefix))
        {
            GenericPrefix::SimplePrefix(*prefix)
        } else {
            GenericPrefix::Generated(rand::random())
        };

        Generics(Rc::new(generic))
    }

    /// Create generic.
    pub fn create_generic(&self, index: usize, kind: Kind) -> Generic {
        Generic {
            prefix: self.clone(),
            index,
            kind,
        }
    }
}

/// Generic representation.
#[derive(Clone, Debug, Serialize)]
pub struct Generic {
    prefix: Generics,
    index: usize,
    #[serde(with = "serde_kind::Kind")]
    kind: Kind,
}

mod serde_kind {
    use serde::{Serialize, Deserialize};

    #[derive(Serialize, Deserialize)]
    #[serde(remote = "libra::file_format::Kind")]
    pub enum Kind {
        All,
        Resource,
        Copyable,
    }
}

impl Generic {
    ///Returns generic name.
    pub fn as_name(&self) -> GenericName {
        GenericName(&self)
    }
}

impl Encode for Generics {
    fn encode<W: Write>(&self, w: &mut W, _indent: usize) -> Result<(), Error> {
        match self.0.as_ref() {
            GenericPrefix::SimplePrefix(p) => w.write_str(p)?,
            GenericPrefix::Generated(p) => write!(w, "TYPE{}", p)?,
        }
        Ok(())
    }
}

impl Encode for Generic {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        self.prefix.encode(w, indent)?;

        if self.index != 0 {
            write!(w, "{}", self.index)?;
        }

        match self.kind {
            Kind::All => { /* no-op */ }
            Kind::Resource => w.write_str(": resource")?,
            Kind::Copyable => w.write_str(": copyable")?,
        };
        Ok(())
    }
}

/// Generic name.
pub struct GenericName<'a>(&'a Generic);

impl<'a> Encode for GenericName<'a> {
    fn encode<W: Write>(&self, w: &mut W, indent: usize) -> Result<(), Error> {
        self.0.prefix.encode(w, indent)?;

        if self.0.index != 0 {
            write!(w, "{}", self.0.index)?;
        }

        Ok(())
    }
}

/// Extract type parameters.
pub fn extract_type_params(params: &[Kind], generics: &Generics) -> Vec<Generic> {
    params
        .iter()
        .enumerate()
        .map(|(i, k)| generics.create_generic(i, *k))
        .collect()
}

/// Write type parameters to writer.
pub fn write_type_parameters<W: Write>(w: &mut W, type_params: &[Generic]) -> Result<(), Error> {
    if !type_params.is_empty() {
        write_array(w, "<", ", ", type_params, ">")?;
    }
    Ok(())
}
