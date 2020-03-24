use dvm_lang::banch32::{libra_into_bech32, bech32_into_libra, replace_bech32_addresses};

#[test]
fn test_match_valid_import_bech32_lines() {
    let sources = "import cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e.Account; import wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6.Account;";
    let replaced_line = replace_bech32_addresses(sources);
    assert_eq!(
        r"import 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.Account; import 0x77616c6c65747300000000002eb8d97a078f3ae572b0ea70362080c3e188a7e6.Account;",
        replaced_line
    );
}

#[test]
fn test_match_arbitrary_import_whitespaces() {
    let line = "import          cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e.Account;";
    let replaced_line = replace_bech32_addresses(line);
    assert_eq!(
        r"import          0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.Account;",
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
    assert_eq!(replace_bech32_addresses(source), source);

    let source = r"
            import 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.LibraAccount;
            import 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.LibraCoin;
            main() {return;}
        ";
    assert_eq!(replace_bech32_addresses(source), source);
}

#[test]
fn test_valid_bech32_libra_address_not_replaced() {
    let source = r"
            import 0x123456789abcdef123456789abcdef123456789abcdef123456789abcdefeeee.Account;
            main() {
                return;
            }
        ";
    assert_eq!(replace_bech32_addresses(source), source);
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
    assert_eq!(replace_bech32_addresses(source), source);
}

#[test]
fn test_match_valid_import_bech32_lines_move() {
    let line = "use cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e::Account; use wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6::Account;";
    let replaced_line = replace_bech32_addresses(line);
    assert_eq!(
        replaced_line,
        r"use 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb::Account; use 0x77616c6c65747300000000002eb8d97a078f3ae572b0ea70362080c3e188a7e6::Account;",
    );
}

#[test]
fn test_leave_libra_addresses_untouched_move() {
    let original_source = r"
            use 0x0::LibraAccount;
            use 0x0::LibraCoin;
            main() {return;}
        ";
    assert_eq!(replace_bech32_addresses(original_source), original_source,);

    let original_source = r"
            use 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb::LibraAccount;
            use 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb::LibraCoin;
            main() {return;}
        ";
    assert_eq!(replace_bech32_addresses(original_source), original_source,);
}

#[test]
fn test_address_as_variable() {
    let source = r"addr = cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e;";
    assert_eq!(
        replace_bech32_addresses(source),
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
        "Malformed bech32: invalid length",
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
fn test_roundtrip_conversion() {
    fn roundtrip(bech32_address: &str) {
        let libra_address = format!("0x{}", bech32_into_libra(bech32_address).unwrap());
        assert_eq!(libra_into_bech32(&libra_address).unwrap(), bech32_address,);
    }
    roundtrip("cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e");
    roundtrip("wallets196udj7s83uaw2u4safcrvgyqc0sc3flxuherp6");
}
