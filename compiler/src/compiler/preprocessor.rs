use twox_hash::XxHash64;
use std::hash::Hasher;
use crate::compiler::bech32::replace_bech32_addresses;

#[macro_export]
macro_rules! pattern {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

pub fn str_xxhash(val: &str) -> u64 {
    let mut hash = XxHash64::default();
    Hasher::write(&mut hash, val.as_bytes());
    Hasher::finish(&hash)
}

pub fn replace_u_literal(code: &str) -> String {
    let mut replaced = code.to_string();
    let regex = pattern!(r#"#".*?""#);

    let replace_list = regex
        .find_iter(code)
        .map(|mat| {
            let content = mat
                .as_str()
                .to_lowercase()
                .chars()
                .skip(2)
                .take(mat.as_str().len() - 3)
                .collect::<String>();

            (mat.range(), format!("{}", str_xxhash(&content)))
        })
        .collect::<Vec<_>>();

    for (range, value) in replace_list.into_iter().rev() {
        replaced.replace_range(range, &value);
    }
    replaced
}

pub fn pre_processing(code: &str) -> String {
    let code = replace_bech32_addresses(code);
    replace_u_literal(&code)
}
