use std::fs;

use libra_types::account_address::AccountAddress;
use maplit::hashmap;
use structopt::StructOpt;

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
    let whitelist = Whitelist::new(hashmap! {
        AccountAddress::default() => vec!["*".to_string()]
    });
    dbg!(is_only_whitelisted_imports(source, address, whitelist));
}
