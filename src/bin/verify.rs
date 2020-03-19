use std::fs;

use libra_types::account_address::AccountAddress;
use maplit::hashmap;
use structopt::StructOpt;

use dvm::vm::{self, validate_bytecode_instructions, WhitelistVerifier, Lang};

#[derive(StructOpt)]
struct Opts {
    fname: String,
}

fn main() {
    let Opts { fname } = Opts::from_args();
    let source = Box::leak(
        fs::read_to_string(fname)
            .expect("Unable to read file")
            .into_boxed_str(),
    );
    let address = AccountAddress::default();
    let whitelist = hashmap! {
        AccountAddress::default() => vec!["LibraAccount".to_string()]
    };

    let script = vm::compile_script(&source, Lang::MvIr, &address);
    if let Err(err) = validate_bytecode_instructions(&script) {
        dbg!(err);
    }

    let whitelister = WhitelistVerifier::new(address, vec![], whitelist);
    if let Err(err) = whitelister.verify_only_whitelisted_modules(&script) {
        dbg!(err);
    }
}
