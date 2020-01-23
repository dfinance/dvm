extern crate lazy_static;

use anyhow::Result;
use bech32::u5;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref BECH32_REGEX: Regex = Regex::new(
        r#"["!#$%&'()*+,\-./0123456789:;<=>?@A-Z\[\\\]^_`a-z{|}~]{1,83}1[A-Z0-9a-z&&[^boi1]]{6,}"#,
    )
    .unwrap();
}

fn vec_u8_into_hex_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{:x?}", byte))
        .collect::<Vec<String>>()
        .join("")
}

pub fn bech32_into_libra_address(address: &str) -> String {
    let (hrp, data_bytes) =
        bech32::decode(address).unwrap_or_else(|_| panic!("Invalid bech32 address {}", address));

    let hrp_bytes = hrp.chars().map(|chr| chr as u8).collect::<Vec<u8>>();
    let hrp = vec_u8_into_hex_string(&hrp_bytes);

    let data_bytes = bech32::convert_bits(&data_bytes, 5, 8, true).unwrap();
    let data = vec_u8_into_hex_string(&data_bytes);

    format!("{:0<24}{}", hrp, data)
}

pub fn libra_address_into_bech32(libra_address: &str) -> Result<String> {
    if libra_address.starts_with("0x") {
        return Err(anyhow!("Strip 0x before passing an address"));
    }
    let bytes = hex::decode(libra_address).unwrap();
    if bytes.len() != 32 {
        return Err(anyhow!(
            "Invalid libra-encoded bech32 length: {}",
            bytes.len()
        ));
    }

    let parts = bytes
        .split(|b| (*b) == 0)
        .filter(|slice| !slice.is_empty())
        .collect::<Vec<&[u8]>>();
    if parts.len() != 2 {
        return Err(anyhow!("Malformed bech32"));
    }
    let hrp = parts[0];
    let hrp = match String::from_utf8((&hrp).to_vec()) {
        Ok(hrp) => hrp,
        Err(err) => {
            return Err(anyhow!(err));
        }
    };
    let data = parts[1];
    if data.len() != 20 {
        return Err(anyhow!("Invalid data part length: {}", data.len()));
    }
    let data_u5 = bech32::convert_bits(data, 8, 5, true)?
        .iter()
        .map(|bit| u5::try_from_u8(bit.to_owned()).unwrap())
        .collect::<Vec<u5>>();
    let encoded = bech32::encode(&hrp, data_u5)?;
    Ok(encoded)
}

pub fn find_and_replace_bech32_addresses(source: &str) -> String {
    let mut transformed_source = source.to_string();
    for mat in BECH32_REGEX.find_iter(source).into_iter() {
        let address = mat.as_str();
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
        let sources = "import cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e.WingsAccount; import bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq.WingsAccount;";
        let replaced_line = find_and_replace_bech32_addresses(sources);
        assert_eq!(
            r"import 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb.WingsAccount; import 0x626300000000000000000000746f8c63f19366129fd563f2366e28f342a16210.WingsAccount;",
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
            import 0x00000111110000011111000001111122.LibraAccount;
            import 0x00000111110000011111000001111122.LibraCoin;
            main() {return;}
        ";
        assert_eq!(find_and_replace_bech32_addresses(source), source);
    }

    #[test]
    fn test_match_valid_import_bech32_lines_move() {
        let line = "use cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e::WingsAccount; use bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq::WingsAccount;";
        let replaced_line = find_and_replace_bech32_addresses(line);
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
        assert_eq!(find_and_replace_bech32_addresses(source), source);

        let source = r"
            use 0x00000111110000011111000001111122::LibraAccount;
            use 0x00000111110000011111000001111122::LibraCoin;
            main() {return;}
        ";
        assert_eq!(find_and_replace_bech32_addresses(source), source);
    }

    #[test]
    fn test_address_as_variable() {
        let source = r"addr = cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e;";
        assert_eq!(
            find_and_replace_bech32_addresses(source),
            r"addr = 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb;"
        )
    }

    #[test]
    fn test_zero_x_not_stripped() {
        let invalid_libra_address =
            "0x636f736d6f730000000000000000000000000000000000000000000000000000";
        assert_eq!(
            libra_address_into_bech32(invalid_libra_address)
                .unwrap_err()
                .to_string(),
            "Strip 0x before passing an address"
        );
    }

    #[test]
    fn test_invalid_libra_address_length() {
        let invalid_libra_address = "636f736d6f73";
        assert_eq!(
            libra_address_into_bech32(invalid_libra_address)
                .unwrap_err()
                .to_string(),
            "Invalid libra-encoded bech32 length: 6"
        );
    }

    #[test]
    fn test_invalid_libra_missing_data_part() {
        let invalid_libra_address =
            "636f736d6f730000000000000000000000000000000000000000000000000000";
        assert_eq!(
            libra_address_into_bech32(invalid_libra_address)
                .unwrap_err()
                .to_string(),
            "Malformed bech32"
        );
    }

    #[test]
    fn test_invalid_libra_missing_hrp_part() {
        let invalid_libra_address =
            "0000000000000000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb";
        assert_eq!(
            libra_address_into_bech32(invalid_libra_address)
                .unwrap_err()
                .to_string(),
            "Malformed bech32"
        );
    }

    #[test]
    fn test_convert_valid_libra_into_bech32() {
        let libra_address = "636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb";
        assert_eq!(
            libra_address_into_bech32(libra_address).unwrap(),
            "cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e"
        );
    }
}
