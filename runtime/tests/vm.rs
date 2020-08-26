use compiler::Compiler;
use ds::MockDataSource;
use lang::{stdlib::zero_std};
use libra::{prelude::*, vm::*};
use dvm_runtime::vm::dvm::Dvm;
use dvm_runtime::vm::types::{Gas, ModuleTx, ScriptTx};
use dvm_runtime::resources::U64Store;

#[test]
fn test_publish_module() {
    let ds = MockDataSource::with_write_set(zero_std());
    let compiler = Compiler::new(ds.clone());
    let vm = Dvm::new(ds.clone(), None);
    let account = AccountAddress::random();

    let program = "module M {}";
    let module = ModuleTx::new(compiler.compile(program, Some(account)).unwrap(), account);
    let output = vm
        .publish_module(Gas::new(1_000_000, 1).unwrap(), module.clone())
        .unwrap();

    let compiled_module = CompiledModule::deserialize(module.code()).unwrap();
    let module_id = compiled_module.self_id();
    assert!(ds.get_module(&module_id).unwrap().is_none());

    ds.merge_write_set(output.write_set);
    assert_ne!(output.gas_used, 0);

    let loaded_module = RemoteCache::get_module(&ds, &module_id).unwrap().unwrap();
    assert_eq!(loaded_module.as_slice(), module.code());

    //try public module duplicate;
    assert_eq!(
        StatusCode::DUPLICATE_MODULE_NAME,
        vm.publish_module(Gas::new(1_000_000, 1).unwrap(), module)
            .unwrap()
            .status
            .major_status()
    );
}

#[test]
fn test_execute_script() {
    let ds = MockDataSource::with_write_set(zero_std());
    let compiler = Compiler::new(ds.clone());
    let vm = Dvm::new(ds.clone(), None);
    let account = AccountAddress::random();

    let module = include_str!("../../test-kit/tests/resources/store.move");
    let module = ModuleTx::new(compiler.compile(module, Some(account)).unwrap(), account);
    ds.merge_write_set(
        vm.publish_module(Gas::new(1_000_000, 1).unwrap(), module)
            .unwrap()
            .write_set,
    );

    let script = format!(
        "
            script {{
            use 0x{}::Store;
            fun main(account: &signer, _account_1: &signer, val: u64) {{
                Store::store_u64(account, val);
            }}
            }}
        ",
        account
    );
    let script = compiler.compile(&script, Some(account)).unwrap();
    let test_value = U64Store { val: 100 };
    let result = vm
        .execute_script(
            Gas::new(1_000_000, 1).unwrap(),
            ScriptTx::new(
                script,
                vec![Value::u64(test_value.val)],
                vec![],
                vec![CORE_CODE_ADDRESS, CORE_CODE_ADDRESS],
            )
            .unwrap(),
        )
        .unwrap();
    assert!(!result.write_set.is_empty());
    let (_, op) = result.write_set.iter().next().unwrap();
    if let WriteOp::Value(blob) = op {
        let value_store: U64Store = lcs::from_bytes(&blob).unwrap();
        assert_eq!(test_value, value_store);
    } else {
        unreachable!();
    }
}
