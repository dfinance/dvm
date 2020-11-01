use libra::account::AccountAddress;

const MODULE: &str = include_str!("../../assets/templates/module.move");
const LOAD_SCRIPT: &str = include_str!("../../assets/templates/load_script.move");
const STORE_SCRIPT: &str = include_str!("../../assets/templates/store_script.move");

const ADDRESS_TEMPLATE: &str = "0x1";
const ID_TEMPLATE: &str = "_ID_";

pub fn module(id: &str, account: &AccountAddress) -> String {
    fill(MODULE, id, account)
}

pub fn store_script(id: &str, account: &AccountAddress) -> String {
    fill(STORE_SCRIPT, id, account)
}

pub fn load_script(id: &str, account: &AccountAddress) -> String {
    fill(LOAD_SCRIPT, id, account)
}

fn fill(src: &str, id: &str, account: &AccountAddress) -> String {
    src.to_string()
        .replace(ADDRESS_TEMPLATE, &format!("0x{}", account))
        .replace(ID_TEMPLATE, id)
}
