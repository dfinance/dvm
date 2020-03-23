use anyhow::Result;
use bech32::u5;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref BECH32_REGEX: Regex = Regex::new(
        r#"[\s=]+(["!#$%&'()*+,\-./0123456789:;<=>?@A-Z\[\\\]^_`a-z{|}~]{1,83}1[A-Z0-9a-z&&[^boi1]]{6,})"#,
    )
    .unwrap();
}

fn vec_u8_into_hex_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| {
            // into 2-length hex strings
            let hex = format!("{:02x?}", byte);
            hex
        })
        .collect::<Vec<String>>()
        .join("")
}

pub fn bech32_into_libra(address: &str) -> Result<String> {
    let (hrp, data_bytes) = bech32::decode(address)?;

    let hrp_bytes = hrp.as_bytes().to_vec();
    let hrp = hex::encode(hrp_bytes);

    let data_bytes = bech32::convert_bits(&data_bytes, 5, 8, true).unwrap();
    let data = vec_u8_into_hex_string(&data_bytes);

    Ok(format!("{:0<24}{}", hrp, data))
}

pub fn libra_into_bech32(libra_address: &str) -> Result<String> {
    ensure!(
        libra_address.starts_with("0x"),
        "Pass address with 0x prefix"
    );
    let libra_address = &libra_address[2..];
    ensure!(libra_address.len() == 64, "Address should be of length 64");

    let (hrp_part, data_part) = libra_address.split_at(24);
    ensure!(data_part.len() == 40, "Data part should be of length 40");

    // hrp
    let hrp_hex = hrp_part.trim_end_matches('0');
    let hrp_bytes = &hex::decode(hrp_hex).unwrap();
    let hrp = hrp_bytes.iter().map(|&c| c as char).collect::<String>();

    // data
    let mut data_bytes = vec![];
    for ind in 0..(data_part.len() / 2) {
        let hex = &data_part[ind * 2..ind * 2 + 2];
        data_bytes.push(hex::decode(hex).unwrap()[0]);
    }
    ensure!(
        data_bytes.len() == 20,
        "Invalid data part length: {}",
        data_bytes.len()
    );
    let data_u5 = bech32::convert_bits(&data_bytes, 8, 5, true)?
        .iter()
        .map(|bit| {
            u5::try_from_u8(bit.to_owned()).expect("Cannot convert u8 into u5 for data index")
        })
        .collect::<Vec<u5>>();

    bech32::encode(&hrp, data_u5).map_err(|err| anyhow!("Malformed bech32: {}", err))
}

pub fn replace_bech32_addresses(source: &str) -> String {
    let mut transformed_source = source.to_string();
    for mat in BECH32_REGEX.captures_iter(source).into_iter() {
        let address = mat.get(1).unwrap().as_str();
        if address.starts_with("0x") {
            // libra match, don't replace
            continue;
        }
        if let Ok(libra_address) = bech32_into_libra(address) {
            transformed_source =
                transformed_source.replace(address, &format!("0x{}", libra_address));
        }
    }
    transformed_source
}
