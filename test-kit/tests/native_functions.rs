use byteorder::{ByteOrder, LittleEndian};
use serde_derive::Serialize;

use dvm_net::api::grpc::VmTypeTag;
use dvm_net::api::grpc::{LcsTag, LcsType, ModuleIdent, StructIdent, VmArgs};
use dvm_test_kit::*;
use dvm_test_kit::TestKit;
use libra::{lcs, prelude::*};
use runtime::resources::*;
use data_source::CurrencyInfo;

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

    let res = test_kit.execute_script(
        script,
        gas_meta(),
        vec![],
        vec![],
        vec![account_address],
        0,
        0,
    );
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
    let res = test_kit.execute_script(script, gas_meta(), args, vec![], vec![account], 0, 0);
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
        0,
        0,
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
    #[allow(clippy::upper_case_acronyms)]
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

fn u128_arg(val: u128) -> VmArgs {
    let mut buf = vec![0; 16];
    LittleEndian::write_u128(&mut buf, val);
    VmArgs {
        r#type: VmTypeTag::U128 as i32,
        value: buf,
    }
}

#[test]
fn test_balance() {
    let test_kit = TestKit::new();

    let addr_1 = AccountAddress::random();
    let addr_2 = AccountAddress::random();
    let init_usdt = 1024;
    let init_pont = 64;
    let init_btc = 13;

    test_kit.set_balance(addr_1, "USDT", init_usdt);
    test_kit.set_balance(addr_1, "XFI", init_pont);
    test_kit.set_balance(addr_1, "BTC", init_btc);

    let res = test_kit.execute_script(
        include_str!("resources/balance.move"),
        gas_meta(),
        vec![u128_arg(init_usdt), u128_arg(init_pont), u128_arg(init_btc)],
        vec![],
        vec![addr_1, addr_2],
        0,
        0,
    );
    test_kit.assert_success(&res);
    test_kit.merge_result(&res);

    assert_eq!(test_kit.get_balance(&addr_1, "USDT"), Some(512));
    assert_eq!(test_kit.get_balance(&addr_1, "XFI"), Some(61));
    assert_eq!(test_kit.get_balance(&addr_1, "BTC"), Some(13));

    assert_eq!(test_kit.get_balance(&addr_2, "USDT"), Some(512));
    assert_eq!(test_kit.get_balance(&addr_2, "XFI"), Some(3));
    assert_eq!(test_kit.get_balance(&addr_2, "BTC"), None);
}

#[test]
fn test_coin_info() {
    let test_kit = TestKit::new();
    let xfi = CurrencyInfo {
        denom: "XFI".as_bytes().to_vec(),
        decimals: 2,
        is_token: true,
        address: CORE_CODE_ADDRESS,
        total_supply: 42,
    };

    let btc = CurrencyInfo {
        denom: "BTC".as_bytes().to_vec(),
        decimals: 10,
        is_token: true,
        address: CORE_CODE_ADDRESS,
        total_supply: 1024,
    };

    test_kit.set_currency_info("XFI", xfi);
    test_kit.set_currency_info("BTC", btc);

    let res = test_kit.execute_script(
        include_str!("resources/currency_info.move"),
        gas_meta(),
        vec![],
        vec![],
        vec![CORE_CODE_ADDRESS],
        0,
        0,
    );
    test_kit.assert_success(&res);
}
