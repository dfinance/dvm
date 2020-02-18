extern crate lazy_static;

use anyhow::Result;
use bech32::u5;
use lazy_static::lazy_static;
use libra_types::access_path::AccessPath;
use libra_types::account_address::AccountAddress;
use regex::Regex;
use crate::compiled_protos::ds_grpc::DsAccessPath;

lazy_static! {
    static ref BECH32_REGEX: Regex = Regex::new(
        r#"[\s=]+(["!#$%&'()*+,\-./0123456789:;<=>?@A-Z\[\\\]^_`a-z{|}~]{1,83}1[A-Z0-9a-z&&[^boi1]]{6,})"#,
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

pub fn libra_access_path_into_ds_access_path(access_path: &AccessPath) -> Result<DsAccessPath> {
    let address = format!("0x{}", access_path.address.to_string());
    let bech32_address = libra_address_string_into_bech32(&address)?;
    let ds_access_path = DsAccessPath {
        address: bech32_address.into_bytes(),
        path: access_path.path.clone(),
    };
    Ok(ds_access_path)
}

pub fn libra_address_string_into_bech32(libra_address: &str) -> Result<String> {
    ensure!(
        libra_address.starts_with("0x"),
        "Pass address with 0x prefix"
    );
    let address = AccountAddress::from_hex_literal(libra_address)?;
    let bytes = address.to_vec();
    let parts = bytes
        .split(|b| (*b) == 0)
        .filter(|slice| !slice.is_empty())
        .collect::<Vec<&[u8]>>();
    ensure!(parts.len() == 2, "Malformed bech32");

    let hrp = String::from_utf8((&parts[0]).to_vec())?;
    let data = parts[1];
    ensure!(data.len() == 20, "Invalid data part length: {}", data.len());

    let data_u5 = bech32::convert_bits(data, 8, 5, true)?
        .iter()
        .map(|bit| u5::try_from_u8(bit.to_owned()).unwrap())
        .collect::<Vec<u5>>();
    let encoded = bech32::encode(&hrp, data_u5)?;
    Ok(encoded)
}

pub fn find_and_replace_bech32_addresses(source: &str) -> String {
    let mut transformed_source = source.to_string();
    for mat in BECH32_REGEX.captures_iter(source).into_iter() {
        let address = mat.get(1).unwrap().as_str();
        if address.starts_with("0x") {
            // libra match, don't replace
            continue;
        }
        let libra_address = bech32_into_libra_address(address);
        transformed_source = transformed_source.replace(address, &format!("0x{}", libra_address));
    }
    transformed_source
}

#[cfg(test)]
mod tests {
    use libra_types::identifier::Identifier;
    use libra_types::language_storage::ModuleId;

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
            use 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb::LibraAccount;
            use 0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb::LibraCoin;
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
    fn test_zero_x_is_stripped() {
        let invalid_libra_address =
            "636f736d6f730000000000000000000000000000000000000000000000000000";
        assert_eq!(
            libra_address_string_into_bech32(invalid_libra_address)
                .unwrap_err()
                .to_string(),
            "Pass address with 0x prefix"
        );
    }

    #[test]
    fn test_invalid_libra_address_length() {
        let invalid_libra_address = "0x636f736d6f73";
        assert_eq!(
            libra_address_string_into_bech32(invalid_libra_address)
                .unwrap_err()
                .to_string(),
            "Malformed bech32"
        );
    }

    #[test]
    fn test_invalid_libra_missing_data_part() {
        let invalid_libra_address =
            "0x636f736d6f730000000000000000000000000000000000000000000000000000";
        assert_eq!(
            libra_address_string_into_bech32(invalid_libra_address)
                .unwrap_err()
                .to_string(),
            "Malformed bech32"
        );
    }

    #[test]
    fn test_invalid_libra_missing_hrp_part() {
        let invalid_libra_address =
            "0x0000000000000000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb";
        assert_eq!(
            libra_address_string_into_bech32(invalid_libra_address)
                .unwrap_err()
                .to_string(),
            "Malformed bech32"
        );
    }

    #[test]
    fn test_convert_valid_libra_into_bech32() {
        let libra_address = "0x636f736d6f730000000000008180b3763b7cef44f142b112cbbe8bffce9d88eb";
        assert_eq!(
            libra_address_string_into_bech32(libra_address).unwrap(),
            "cosmos1sxqtxa3m0nh5fu2zkyfvh05tll8fmz8tk2e22e"
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
            99, 111, 115, 109, 111, 115, 49, 115, 120, 113, 116, 120, 97, 51, 109, 48, 110, 104,
            53, 102, 117, 50, 122, 107, 121, 102, 118, 104, 48, 53, 116, 108, 108, 56, 102, 109,
            122, 56, 116, 107, 50, 101, 50, 50, 101,
        ];
        let result_path_bytes = vec![
            0, 247, 189, 211, 137, 27, 67, 193, 0, 32, 177, 135, 204, 108, 162, 43, 87, 115, 88,
            188, 70, 68, 252, 200, 126, 150, 210, 164, 248, 77, 64, 188, 158,
        ];
        assert_eq!(
            libra_access_path_into_ds_access_path(&access_path).unwrap(),
            DsAccessPath {
                address: result_address_bytes,
                path: result_path_bytes
            }
        );
    }
}
