use anyhow::Result;
use libra::vm::access::ScriptAccess;
use libra::vm::file_format::{Bytecode, CompiledScript};

pub fn validate_bytecode_instructions(script: &CompiledScript) -> Result<()> {
    let instructions = &script.main().code.code;
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
            | Bytecode::LdAddr(_)
            | Bytecode::LdByteArray(_)
            // assignments
            | Bytecode::StLoc(_)
            | Bytecode::CopyLoc(_)
            | Bytecode::MoveLoc(_)
            // misc
            | Bytecode::Call(_, _) => Ok(()),
            _ => Err(anyhow!("Unsafe bytecode instruction")),
        }?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use libra::libra_types::account_address::AccountAddress;
    use crate::compiler::test::compile_script;

    #[test]
    fn test_trivial_script_is_accepted() {
        let source = r"
            main() {return;}
        ";
        let compiled = compile_script(source, None, &AccountAddress::default());
        validate_bytecode_instructions(&compiled).unwrap();
    }

    #[test]
    fn test_assignment_is_accepted() {
        let source = r"
            main() {
                let a: u64;
                a = 1;
                return;
            }
        ";
        let compiled = compile_script(source, None, &AccountAddress::default());
        validate_bytecode_instructions(&compiled).unwrap();
    }

    #[test]
    fn test_call_module_is_accepted() {
        let source = r"
            import 0x0.Account;

            main() {
               Account.create_account(0x0);
               return;
            }
        ";
        let compiled = compile_script(source, None, &AccountAddress::default());
        validate_bytecode_instructions(&compiled).unwrap();
    }

    #[test]
    fn test_if_is_forbidden() {
        let source = r"
            main() {
                if (true) {
                    return;
                }
                return;
            }
        ";
        let compiled = compile_script(source, None, &AccountAddress::default());
        validate_bytecode_instructions(&compiled).unwrap_err();
    }

    #[test]
    fn test_loop_is_forbidden() {
        let source = r"
            main() {
                loop {
                    break;
                }
                return;
            }
        ";
        let compiled = compile_script(source, None, &AccountAddress::default());
        validate_bytecode_instructions(&compiled).unwrap_err();
    }
}
