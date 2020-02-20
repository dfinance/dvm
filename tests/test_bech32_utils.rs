use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use libra_types::identifier::Identifier;
use libra_types::language_storage::ModuleId;

use move_vm_in_cosmos::compiled_protos::ds_grpc::DsAccessPath;
use move_vm_in_cosmos::vm::bech32_utils::{
    find_and_replace_bech32_addresses, libra_into_bech32, libra_access_path_into_ds_access_path,
    bech32_into_libra,
};

#[test]
fn test_match_valid_import_bech32_lines() {
    let sources = "import cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e.WingsAccount; import wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6.WingsAccount;";
    let replaced_line = find_and_replace_bech32_addresses(sources);
    assert_eq!(
        r"import 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.WingsAccount; import 0x77616c6c65747300000000002eb8d97a078f3ae572b0ea70362080c3e188a7e6.WingsAccount;",
        replaced_line
    );
}

#[test]
fn test_match_arbitrary_import_whitespaces() {
    let line = "import          cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e.WingsAccount;";
    let replaced_line = find_and_replace_bech32_addresses(line);
    assert_eq!(
        r"import          0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.WingsAccount;",
        replaced_line
    );
}

#[test]
fn test_leave_libra_addresses_untouched_mvir() {
    let source = r"
            import 0x0.LibraAccount;
            import 0x0.LibraCoin;
            main() {return;}
        ";
    assert_eq!(find_and_replace_bech32_addresses(source), source);

    let source = r"
            import 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.LibraAccount;
            import 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.LibraCoin;
            main() {return;}
        ";
    assert_eq!(find_and_replace_bech32_addresses(source), source);
}

#[test]
fn test_valid_bech32_libra_address_not_replaced() {
    let source = r"
            import 0x123456789abcdef123456789abcdef123456789abcdef123456789abcdefeeee.WingsAccount;
            main() {
                return;
            }
        ";
    assert_eq!(find_and_replace_bech32_addresses(source), source);
}

#[test]
fn test_do_not_replace_invalid_bech32_addresses() {
    let source = r"
        import 0x0.Dbg;
        main(l: u256) {
            let r: u256;
            r = 1233124232344232214519u245;
            Dbg.dump_u256(l + r);
            return;
        };
    ";
    assert_eq!(find_and_replace_bech32_addresses(source), source);
}

#[test]
fn test_match_valid_import_bech32_lines_move() {
    let line = "use cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e::WingsAccount; use wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6::WingsAccount;";
    let replaced_line = find_and_replace_bech32_addresses(line);
    assert_eq!(
        replaced_line,
        r"use 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb::WingsAccount; use 0x77616c6c65747300000000002eb8d97a078f3ae572b0ea70362080c3e188a7e6::WingsAccount;",
    );
}

#[test]
fn test_leave_libra_addresses_untouched_move() {
    let original_source = r"
            use 0x0::LibraAccount;
            use 0x0::LibraCoin;
            main() {return;}
        ";
    assert_eq!(
        find_and_replace_bech32_addresses(original_source),
        original_source,
    );

    let original_source = r"
            use 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb::LibraAccount;
            use 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb::LibraCoin;
            main() {return;}
        ";
    assert_eq!(
        find_and_replace_bech32_addresses(original_source),
        original_source,
    );
}

#[test]
fn test_address_as_variable() {
    let source = r"addr = cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e;";
    assert_eq!(
        find_and_replace_bech32_addresses(source),
        r"addr = 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb;",
    )
}

#[test]
fn test_zero_x_is_stripped() {
    let invalid_libra_address = "636f736d6f730000000000000000000000000000000000000000000000000000";
    assert_eq!(
        libra_into_bech32(invalid_libra_address)
            .unwrap_err()
            .to_string(),
        "Pass address with 0x prefix",
    );
}

#[test]
fn test_invalid_libra_address_length() {
    let invalid_libra_address = "0x636f736d6f73";
    assert_eq!(
        libra_into_bech32(invalid_libra_address)
            .unwrap_err()
            .to_string(),
        "Address should be of length 64",
    );
}

#[test]
fn test_invalid_libra_missing_hrp_part() {
    let invalid_libra_address =
        "0x0000000000000000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb";
    assert_eq!(
        libra_into_bech32(invalid_libra_address)
            .unwrap_err()
            .to_string(),
        "Malformed bech32",
    );
}

#[test]
fn test_convert_valid_libra_into_bech32() {
    let libra_address = "0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb";
    assert_eq!(
        libra_into_bech32(libra_address).unwrap(),
        "cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e",
    );
}

#[test]
fn test_libra_access_path_into_data_source_request() {
    let libra_address = AccountAddress::from_hex_literal(
        "0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb",
    )
    .unwrap();
    let requested_module_id = ModuleId::new(
        libra_address,
        Identifier::new("WingsAccount".to_string().into_boxed_str()).unwrap(),
    );
    let access_path = AccessPath::from(&requested_module_id);

    let result_address_bytes = vec![
        99, 111, 115, 109, 111, 115, 49, 115, 120, 113, 116, 120, 97, 51, 109, 48, 110, 104, 53,
        102, 117, 50, 122, 107, 121, 102, 118, 104, 48, 53, 116, 108, 108, 56, 102, 109, 122, 56,
        116, 107, 50, 101, 50, 50, 101,
    ];
    let result_path_bytes = vec![
        0, 247, 189, 211, 137, 27, 67, 193, 0, 32, 177, 135, 204, 108, 162, 43, 87, 115, 88, 188,
        70, 68, 252, 200, 126, 150, 210, 164, 248, 77, 64, 188, 158,
    ];
    assert_eq!(
        libra_access_path_into_ds_access_path(&access_path).unwrap(),
        DsAccessPath {
            address: result_address_bytes,
            path: result_path_bytes
        }
    );
}

#[test]
fn test_roundtrip_conversion() {
    fn roundtrip(bech32_address: &str) {
        let libra_address = format!("0x{}", bech32_into_libra(bech32_address).unwrap());
        assert_eq!(libra_into_bech32(&libra_address).unwrap(), bech32_address,);
    }
    roundtrip("cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e");
    roundtrip("wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6");
}
