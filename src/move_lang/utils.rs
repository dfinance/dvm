extern crate lazy_static;

use lazy_static::lazy_static;
use regex::Regex;

use crate::test_kit::Lang;

lazy_static! {
    static ref BECH32_MVIR_REGEX: Regex = Regex::new(
        r#"(?P<prefix>["!#$%&'()*+,\-./0123456789:;<=>?@A-Z\[\\\]^_`a-z{|}~]{1,83}1[A-Z0-9a-z&&[^boi1]]{6,})\.[a-zA-Z0-9]+"#,
    ).unwrap();
    static ref BECH32_MOVE_REGEX: Regex = Regex::new(
        r#"(?P<prefix>["!#$%&'()*+,\-./0123456789:;<=>?@A-Z\[\\\]^_`a-z{|}~]{1,83}1[A-Z0-9a-z&&[^boi1]]{6,})::[a-zA-Z0-9]+"#,
    ).unwrap();
}

fn vec_u8_into_hex_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{:x?}", byte))
        .collect::<Vec<String>>()
        .join("")
}

fn bech32_into_libra_address(address: &str) -> String {
    let (hrp, data_bytes) =
        bech32::decode(address).unwrap_or_else(|_| panic!("Invalid bech32 address {}", address));

    let hrp_bytes = hrp.chars().map(|chr| chr as u8).collect::<Vec<u8>>();
    let hrp = vec_u8_into_hex_string(&hrp_bytes);

    let data_bytes = bech32::convert_bits(&data_bytes, 5, 8, true).unwrap();
    let data = vec_u8_into_hex_string(&data_bytes);

    format!("{:0<24}{}", hrp, data)
}

pub fn replace_bech32_addresses(source: &str, lang: Lang) -> String {
    let mut transformed_source = source.to_string();
    let addresses_regex: Regex = match lang {
        Lang::MvIr => BECH32_MVIR_REGEX.to_owned(),
        Lang::Move => BECH32_MOVE_REGEX.to_owned(),
    };
    for mat in addresses_regex.captures_iter(source).into_iter() {
        let address = mat.name("prefix").unwrap().as_str();
        let libra_address = bech32_into_libra_address(address);
        transformed_source = transformed_source.replace(address, &format!("0x{}", libra_address));
    }
    transformed_source
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_valid_import_bech32_lines() {
        let line = "import cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e.WingsAccount; import bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq.WingsAccount;";
        let replaced_line = replace_bech32_addresses(line, Lang::MvIr);
        assert_eq!(
            r"import 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.WingsAccount; import 0x626300000000000000000000746f8c63f19366129fd563f2366e28f342a16210.WingsAccount;",
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
        assert_eq!(replace_bech32_addresses(source, Lang::MvIr), source);

        let source = r"
            import 0x00000111110000011111000001111122.LibraAccount;
            import 0x00000111110000011111000001111122.LibraCoin;
            main() {return;}
        ";
        assert_eq!(replace_bech32_addresses(source, Lang::MvIr), source);
    }

    #[test]
    fn test_match_valid_import_bech32_lines_move() {
        let line = "use cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e::WingsAccount; use bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq::WingsAccount;";
        let replaced_line = replace_bech32_addresses(line, Lang::Move);
        assert_eq!(
            r"use 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb::WingsAccount; use 0x626300000000000000000000746f8c63f19366129fd563f2366e28f342a16210::WingsAccount;",
            replaced_line
        );
    }

    #[test]
    fn test_leave_libra_addresses_untouched_move() {
        let source = r"
            use 0x0::LibraAccount;
            use 0x0::LibraCoin;
            main() {return;}
        ";
        assert_eq!(replace_bech32_addresses(source, Lang::Move), source);

        let source = r"
            use 0x00000111110000011111000001111122::LibraAccount;
            use 0x00000111110000011111000001111122::LibraCoin;
            main() {return;}
        ";
        assert_eq!(replace_bech32_addresses(source, Lang::Move), source);
    }
}
