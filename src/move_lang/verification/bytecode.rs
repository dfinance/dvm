use std::collections::HashSet;

use anyhow::Result;
use libra_types::account_address::AccountAddress;
use vm::file_format::{Bytecode, CompiledScript};

use crate::test_kit::Lang;

pub(crate) fn compile_script(
    source: &str,
    lang: Lang,
    sender_address: &AccountAddress,
) -> CompiledScript {
    CompiledScript::deserialize(&lang.compiler().build_script(source, sender_address)).unwrap()
}

struct InstructionsVerifier {
    valid_instructions: HashSet<Bytecode>,
}

impl InstructionsVerifier {
    fn new(valid_instructions: HashSet<Bytecode>) -> Self {
        InstructionsVerifier { valid_instructions }
    }

    pub fn ensure_only_valid_instructions(script: &CompiledScript) -> Result<()> {
        Ok(())
    }
}
