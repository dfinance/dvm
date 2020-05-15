use libra::libra_types::account_address::AccountAddress;
use bech32::{encode, ToBase32};
use dvm_compiler::bech32::{libra_into_bech32, replace_bech32_addresses, HRP, bech32_into_libra};

pub fn make_bach32() -> String {
    encode(HRP, rand::random::<[u8; 20]>().to_base32()).unwrap()
}

#[test]
fn test_match_valid_import_bech32_lines() {
    let sources = "import wallet1me0cdn52672y7feddy7tgcj6j4dkzq2su745vh.Account; import wallet1zhw7vn8stj4zu7jgjalyunyhn8462pwnu0v252.Account;";
    let replaced_line = replace_bech32_addresses(sources);
    assert_eq!(
        r"import 0xde5f86ce8ad7944f272d693cb4625a955b61015000000000.Account; import 0x15dde64cf05caa2e7a48977e4e4c9799eba505d300000000.Account;",
        replaced_line
    );
}

#[test]
fn test_match_arbitrary_import_whitespaces() {
    let line = "import          wallet1me0cdn52672y7feddy7tgcj6j4dkzq2su745vh.Account;";
    let replaced_line = replace_bech32_addresses(line);
    assert_eq!(
        r"import          0xde5f86ce8ad7944f272d693cb4625a955b61015000000000.Account;",
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
            import 0x0000000003ea59d310ab43bde44d99ec216ab46abb766a9f.Account;
            import 0x0000000003ea59d310ab43bde44d99ec216ab46abb766a9f.Coin;
            main() {return;}
        ";
    assert_eq!(replace_bech32_addresses(source), source);
}

#[test]
fn test_valid_bech32_libra_address_not_replaced() {
    let source = r"
            import 0x0000000003ea59d310ab43bde44d99ec216ab46abb766a9f.Account;
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
    let line = "use wallet1me0cdn52672y7feddy7tgcj6j4dkzq2su745vh::Account; use wallet1me0cdn52672y7feddy7tgcj6j4dkzq2su745vh::Account;";
    let replaced_line = replace_bech32_addresses(line);
    assert_eq!(
        replaced_line,
        r"use 0xde5f86ce8ad7944f272d693cb4625a955b61015000000000::Account; use 0xde5f86ce8ad7944f272d693cb4625a955b61015000000000::Account;",
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
    let source = r"addr = wallet1me0cdn52672y7feddy7tgcj6j4dkzq2su745vh;";
    assert_eq!(
        replace_bech32_addresses(source),
        r"addr = 0xde5f86ce8ad7944f272d693cb4625a955b61015000000000;",
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
fn test_convert_valid_libra_into_bech32() {
    let libra_address = "0xde5f86ce8ad7944f272d693cb4625a955b61015000000000";
    assert_eq!(
        libra_into_bech32(libra_address).unwrap(),
        "wallet1me0cdn52672y7feddy7tgcj6j4dkzq2su745vh",
    );
}

#[test]
fn test_roundtrip_conversion() {
    fn roundtrip(bech32_address: &str) {
        let libra_address = format!("0x{}", bech32_into_libra(bech32_address).unwrap());
        AccountAddress::from_hex_literal(&libra_address).unwrap();
        assert_eq!(libra_into_bech32(&libra_address).unwrap(), bech32_address,);
    }
    roundtrip(&make_bach32());
    roundtrip(&make_bach32());
}
