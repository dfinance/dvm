use libra::{prelude::*, lcs};
use byteorder::{LittleEndian, ByteOrder};

use dvm_net::api::grpc::vm_grpc::{VmArgs, VmTypeTag, ModuleIdent, LcsTag, StructIdent, LcsType};
use dvm_test_kit::TestKit;
use dvm_test_kit::*;
use serde::Serialize;
use runtime::resources::*;

#[test]
fn test_native_function() {
    let test_kit = TestKit::new();
    test_kit.add_std_module(include_str!("resources/store.move"));

    let script = "
        script {
        use 0x1::Store;

        fun main(account: &signer) {
            Store::store_address(account, 0x110);
        }
        }
    ";

    let account_address = account("0x110");

    let res = test_kit.execute_script(script, gas_meta(), vec![], vec![], vec![account_address]);
    test_kit.assert_success(&res);
    let value: AddressStore = lcs::from_bytes(&res.write_set[0].value).unwrap();
    assert_eq!(value.val, account_address);
}

#[test]
fn test_register_token_info() {
    let test_kit = TestKit::empty();
    test_kit.add_std_module(include_str!("resources/dfinance.move"));

    let script = "\
        script {
        use 0x1::Dfinance;

        fun main(t_value: u64) {
            Dfinance::store_info<Dfinance::SimpleCoin>(t_value);
        }
        }
    ";

    let account = account("0x110");

    let t_value = 13;
    let mut buf = vec![0; 8];
    LittleEndian::write_u64(&mut buf, t_value);
    let args = vec![VmArgs {
        r#type: VmTypeTag::U64 as i32,
        value: buf,
    }];
    let res = test_kit.execute_script(script, gas_meta(), args, vec![], vec![account]);
    test_kit.assert_success(&res);
    let value: U64Store = lcs::from_bytes(&res.write_set[0].value).unwrap();
    assert_eq!(t_value, value.val);
}

#[test]
fn test_events() {
    let test_kit = TestKit::new();
    test_kit.add_std_module(include_str!("resources/currency.move"));
    test_kit.add_std_module(include_str!("resources/event_proxy.move"));

    let script = "\
        script {
        use 0x1::Event;
        use 0x1::Signer;
        use 0x1::Currency;
        use 0x1::EventProxy;

        fun main<Curr: copyable>(account: &signer) {
            let _addr = Signer::address_of(account);
            Event::emit<Currency::Value<Curr>>(account, Currency::make_currency<Curr>(100));
            EventProxy::store<Currency::BTC>(account, Currency::make_btc(101));
        }
        }
    ";
    let sender = account("0x110");
    let res = test_kit.execute_script(
        script,
        gas_meta(),
        vec![],
        vec![StructIdent {
            address: CORE_CODE_ADDRESS.to_vec(),
            module: "Currency".to_string(),
            name: "ETH".to_string(),
            type_params: vec![],
        }],
        vec![sender],
    );
    test_kit.assert_success(&res);

    assert_eq!(res.events.len(), 2);

    let script_event = &res.events[0];
    let proxy_event = &res.events[1];
    assert_eq!(script_event.sender_module, None);
    assert_eq!(
        proxy_event.sender_module,
        Some(ModuleIdent {
            address: CORE_CODE_ADDRESS.to_vec(),
            name: "EventProxy".to_string(),
        })
    );

    assert_eq!(script_event.sender_address, sender.to_vec());
    assert_eq!(proxy_event.sender_address, script_event.sender_address);

    assert_eq!(
        script_event.event_type,
        Some(LcsTag {
            type_tag: LcsType::LcsStruct as i32,
            vector_type: None,
            struct_ident: Some(StructIdent {
                address: CORE_CODE_ADDRESS.to_vec(),
                module: "Currency".to_string(),
                name: "Value".to_string(),
                type_params: vec![LcsTag {
                    type_tag: LcsType::LcsStruct as i32,
                    vector_type: None,
                    struct_ident: Some(StructIdent {
                        address: CORE_CODE_ADDRESS.to_vec(),
                        module: "Currency".to_string(),
                        name: "ETH".to_string(),
                        type_params: vec![],
                    }),
                }],
            }),
        })
    );
    assert_eq!(
        proxy_event.event_type,
        Some(LcsTag {
            type_tag: LcsType::LcsStruct as i32,
            vector_type: None,
            struct_ident: Some(StructIdent {
                address: CORE_CODE_ADDRESS.to_vec(),
                module: "Currency".to_string(),
                name: "BTC".to_string(),
                type_params: vec![],
            }),
        })
    );

    #[derive(Serialize)]
    struct BTC {
        value: u64,
    }

    assert_eq!(
        script_event.event_data,
        lcs::to_bytes(&BTC { value: 100 }).unwrap()
    );
    assert_eq!(
        proxy_event.event_data,
        lcs::to_bytes(&BTC { value: 101 }).unwrap()
    );
}
