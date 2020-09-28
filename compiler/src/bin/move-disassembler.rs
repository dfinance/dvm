extern crate clap;

use std::path::PathBuf;
use clap::Clap;
use anyhow::Error;
use dvm_compiler::disassembler;

#[derive(Clap, Debug)]
#[clap(name = "Move disassembler")]
struct Opt {
    #[clap(about = "Path to input file", long, short)]
    /// Path to compiled Move binary
    input: PathBuf,
}

fn main() -> Result<(), Error> {
    // let current_dir = env::current_dir().unwrap();
    let opts = Opt::parse();

    let input_path = std::fs::canonicalize(opts.input)?;
    let input_bytes = std::fs::read(input_path)?;

    let cfg = disassembler::Config {
        light_version: false,
    };
    let out = disassembler::disasm_str(&input_bytes, cfg)?;

    println!("{}", out);

    Ok(())
}
