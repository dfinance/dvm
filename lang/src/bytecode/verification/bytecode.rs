use anyhow::Result;
use libra::libra_vm::access::ScriptAccess;
use libra::libra_vm::file_format::{Bytecode, CompiledScript};

pub fn validate_bytecode_instructions(script: &CompiledScript) -> Result<()> {
    let instructions = &script.code().code;
    for inst in instructions {
        match inst {
            Bytecode::Pop
            | Bytecode::Ret
            // values
            | Bytecode::LdU8(_)
            | Bytecode::LdU64(_)
            | Bytecode::LdU128(_)
            | Bytecode::LdTrue
            | Bytecode::LdFalse
            | Bytecode::LdConst(_)
            // assignments
            | Bytecode::StLoc(_)
            | Bytecode::CopyLoc(_)
            | Bytecode::MoveLoc(_)
            // misc
            | Bytecode::Call(_) => Ok(()),
            _ => Err(anyhow!("Unsafe bytecode instruction")),
        }?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::verification::whitelist::tests::compile_script;
    use libra::move_core_types::language_storage::CORE_CODE_ADDRESS;

    #[test]
    fn test_trivial_script_is_accepted() {
        let source = r"
            script {
            fun main() {}
            }
        ";
        let compiled = compile_script(source, vec![], &CORE_CODE_ADDRESS);
        validate_bytecode_instructions(&compiled).unwrap();
    }

    #[test]
    fn test_assignment_is_accepted() {
        let source = r"
            script {
            fun main() {
                let _a = 1;
            }
            }
        ";
        let compiled = compile_script(source, vec![], &CORE_CODE_ADDRESS);
        validate_bytecode_instructions(&compiled).unwrap();
    }

    #[test]
    fn test_call_module_is_accepted() {
        let empty = include_str!("../../../tests/resources/empty.move");

        let source = r"
            script {
            use 0x1::Empty;

            fun main() {
               Empty::create();
            }
            }
        ";
        let compiled = compile_script(
            source,
            vec![(empty, &CORE_CODE_ADDRESS)],
            &CORE_CODE_ADDRESS,
        );
        validate_bytecode_instructions(&compiled).unwrap();
    }

    #[test]
    fn test_if_is_forbidden() {
        let source = r"
            script {
            fun main() {
                if (true) {
                }
            }
            }
        ";
        let compiled = compile_script(source, vec![], &CORE_CODE_ADDRESS);
        validate_bytecode_instructions(&compiled).unwrap_err();
    }

    #[test]
    fn test_loop_is_forbidden() {
        let source = r"
            script {
            fun main() {
                loop {
                }
            }
            }
        ";
        let compiled = compile_script(source, vec![], &CORE_CODE_ADDRESS);
        validate_bytecode_instructions(&compiled).unwrap_err();
    }
}
