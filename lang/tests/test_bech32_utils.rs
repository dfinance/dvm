use dvm_lang::banch32::{libra_into_bech32, bech32_into_libra, replace_bech32_addresses};
use libra::libra_types::account_address::AccountAddress;
use bech32::{encode, ToBase32};

pub fn make_bach32() -> String {
    encode("df", rand::random::<[u8; 20]>().to_base32()).unwrap()
}

#[test]
fn test_match_valid_import_bech32_lines() {
    let sources = "import df1pfk58n7j62uenmam7f9ncu6qnffc2q5dpwuute.Account; import df1vaj8palf3xcv7q00gzymeu984d6je9vncqvxlx.Account;";
    let replaced_line = replace_bech32_addresses(sources);
    assert_eq!(
        r"import 0x646600000a6d43cfd2d2b999efbbf24b3c73409a5385028d.Account; import 0x64660000676470f7e989b0cf01ef4089bcf0a7ab752c9593.Account;",
        replaced_line
    );
}

#[test]
fn test_match_arbitrary_import_whitespaces() {
    let line = "import          df1pfk58n7j62uenmam7f9ncu6qnffc2q5dpwuute.Account;";
    let replaced_line = replace_bech32_addresses(line);
    assert_eq!(
        r"import          0x646600000a6d43cfd2d2b999efbbf24b3c73409a5385028d.Account;",
        replaced_line
    );
}

#[test]
fn test_leave_libra_addresses_untouched_mvir() {
    let source = r"
            import 0x0.Account;
            import 0x0.Coin;
            main() {return;}
        ";
    assert_eq!(replace_bech32_addresses(source), source);

    let source = r"
            import 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.Account;
            import 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.Coin;
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
    let line = "use df1pfk58n7j62uenmam7f9ncu6qnffc2q5dpwuute::Account; use df1vaj8palf3xcv7q00gzymeu984d6je9vncqvxlx::Account;";
    let replaced_line = replace_bech32_addresses(line);
    assert_eq!(
        replaced_line,
        r"use 0x646600000a6d43cfd2d2b999efbbf24b3c73409a5385028d::Account; use 0x64660000676470f7e989b0cf01ef4089bcf0a7ab752c9593::Account;",
    );
}

#[test]
fn test_leave_libra_addresses_untouched_move() {
    let original_source = r"
            use 0x0::Account;
            use 0x0::Coin;
            main() {return;}
        ";
    assert_eq!(replace_bech32_addresses(original_source), original_source,);

    let original_source = r"
            use 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb::Account;
            use 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb::Coin;
            main() {return;}
        ";
    assert_eq!(replace_bech32_addresses(original_source), original_source,);
}

#[test]
fn test_address_as_variable() {
    let source = r"addr = df1pfk58n7j62uenmam7f9ncu6qnffc2q5dpwuute;";
    assert_eq!(
        replace_bech32_addresses(source),
        r"addr = 0x646600000a6d43cfd2d2b999efbbf24b3c73409a5385028d;",
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
        "Address should be of length 50",
    );
}

#[test]
fn test_invalid_libra_missing_hrp_part() {
    let invalid_libra_address = "0x000000008180b3763b7cef44f142b112cbbe8bffce9d88eb";
    assert_eq!(
        libra_into_bech32(invalid_libra_address)
            .unwrap_err()
            .to_string(),
        "invalid length",
    );
}

#[test]
fn test_convert_valid_libra_into_bech32() {
    let libra_address = "0x646600000a6d43cfd2d2b999efbbf24b3c73409a5385028d";
    assert_eq!(
        libra_into_bech32(libra_address).unwrap(),
        "df1pfk58n7j62uenmam7f9ncu6qnffc2q5dpwuute",
    );
}

#[test]
fn test_roundtrip_conversion() {
    fn roundtrip(bech32_address: &str) {
        let libra_address = format!("0x{}", bech32_into_libra(bech32_address).unwrap());
        AccountAddress::from_hex_literal(dbg!(&libra_address)).unwrap();
        assert_eq!(libra_into_bech32(&libra_address).unwrap(), bech32_address,);
    }
    roundtrip(dbg!(&make_bach32()));
    roundtrip(&make_bach32());
}
