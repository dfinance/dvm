use anyhow::Result;
use bech32::u5;
use lazy_static::lazy_static;
use regex::Regex;

pub static HRP: &str = "wallet";

lazy_static! {
    static ref BECH32_REGEX: Regex = Regex::new(
        r#"[\s=]+(["!#$%&'()*+,\-./0123456789:;<=>?@A-Z\[\\\]^_`a-z{|}~]{1,83}1[A-Z0-9a-z&&[^boi1]]{6,})"#,
    )
    .unwrap();
}

pub fn bech32_into_libra(address: &str) -> Result<String> {
    let (_, data_bytes) = bech32::decode(address)?;
    let data = bech32::convert_bits(&data_bytes, 5, 8, true)?;
    Ok(hex::encode(&data))
}

pub fn libra_into_bech32(libra_address: &str) -> Result<String> {
    ensure!(
        libra_address.starts_with("0x"),
        "Pass address with 0x prefix"
    );
    ensure!(libra_address.len() == 42, "Address should be of length 42");
    let data = hex::decode(&libra_address[2..])?;
    let data = bech32::convert_bits(&data, 8, 5, true)?
        .into_iter()
        .map(u5::try_from_u8)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(bech32::encode(&HRP, data)?)
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
