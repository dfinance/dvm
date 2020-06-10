use crate::mv::bech32::replace_bech32_addresses;

/// Preprocess move code.
pub fn pre_processing(code: &str) -> String {
    replace_bech32_addresses(code)
}
