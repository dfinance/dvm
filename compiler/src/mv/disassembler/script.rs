use crate::mv::disassembler::Encode;
use anyhow::Error;

pub struct Script {}

impl Encode for Script {
    fn encode<W>(&self, w: &mut W, indent: u8) -> Result<(), Error> {
        unimplemented!()
    }
}
