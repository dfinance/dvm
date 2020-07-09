use std::path::PathBuf;
use libra::prelude::*;
use clap::Clap;
use std::collections::HashMap;
use enum_iterator::IntoEnumIterator;
use anyhow::Result;
use std::fs::OpenOptions;
use std::io::Write;

/// Status table generator.
#[derive(Clap)]
#[clap(name = "status-table")]
struct Opts {
    /// Optional path to the output file.
    /// If not passed, result will be printed to stdout.
    #[clap(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
}

fn main() {
    let opts = Opts::parse();
    let status_table = status_table_json().unwrap();
    if let Some(output) = opts.output {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&output)
            .unwrap();
        f.set_len(0).unwrap();
        f.write_all(status_table.as_bytes()).unwrap();
    } else {
        println!("{}", status_table);
    }
}

fn status_table_json() -> Result<String> {
    Ok(serde_json::to_string_pretty(
        &StatusCode::into_enum_iter()
            .map(|code| (code as u64, format!("{:?}", code)))
            .collect::<HashMap<_, _>>(),
    )?)
}
