extern crate clap;

use std::path::PathBuf;
use std::fs::{File, canonicalize, read};
use std::io::Write;
use clap::Clap;
use anyhow::Error;
use dvm_compiler::disassembler;

#[derive(Clap, Debug)]
#[clap(name = "Move decompiler")]
struct Opt {
    #[clap(about = "Path to input file", long, short)]
    /// Path to compiled Move binary
    input: PathBuf,

    #[clap(about = "Path to output file", long, short)]
    /// Optional path to output file.
    /// Prints results to stdout by default.
    output: Option<PathBuf>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{}", e);
    }
}

fn run() -> Result<(), Error> {
    let opts = Opt::parse();

    let input_path = canonicalize(opts.input)?;
    let input_bytes = read(input_path)?;

    let cfg = disassembler::Config {
        light_version: false,
    };
    let out = disassembler::disasm_str(&input_bytes, cfg)?;

    if let Some(output_path) = opts.output {
        File::create(output_path)?.write_all(out.as_bytes())?;
    } else {
        println!("{}", out);
    }

    Ok(())
}
