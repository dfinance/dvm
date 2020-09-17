use std::{fs, io, path::PathBuf};
use clap::Clap;
use serde_json::{to_string, to_string_pretty};
use lang::stdlib::{build_external_std, Stdlib, WS};
use std::path::Path;
use anyhow::Error;
use std::collections::HashMap;

#[derive(Clap)]
#[clap(name = "stdlib-builder")]
struct Opts {
    /// Path to the directory with the standard library.
    #[clap()]
    source_dir: String,
    /// Optional path to the output file.
    /// If not passed, result will be printed to stdout.
    #[clap(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    #[clap(short = 'v', long = "verbose")]
    /// Verbose mode flag.
    /// Enables debug printing of internals including used modules.
    debug_print: bool,
    /// Enables pretty printing of all output including debug-prints if it enabled.
    #[clap(short)]
    pretty_print: bool,
}

fn main() {
    let opts = Opts::parse();

    let entries = fs::read_dir(&opts.source_dir)
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .filter(|path| {
            if match path {
                Ok(path) => path.extension().map(|ext| ext == "move").unwrap_or(false),
                Err(_) => true,
            } {
                true
            } else {
                println!(
                    "Skip file: {:?}",
                    path.as_ref().unwrap().file_name().unwrap()
                );
                false
            }
        })
        .collect::<Result<Vec<_>, io::Error>>()
        .unwrap();

    if opts.debug_print {
        println!(
            "Modules: {:#?}",
            entries
                .iter()
                .filter_map(|p| { p.file_name().map(|oss| oss.to_str()).flatten() })
                .collect::<Vec<_>>()
        )
    }

    let modules = entries
        .iter()
        .map(|e| load_module(e))
        .collect::<Result<HashMap<String, String>, _>>()
        .unwrap();

    let vm_value = build_external_std(Stdlib { modules }).unwrap();

    // Serialize
    let ws = if opts.pretty_print {
        to_string_pretty(&WS::from(vm_value))
    } else {
        to_string(&WS::from(vm_value))
    }
    .expect("Cannot serialize results to json.");

    // Export the output
    if let Some(path) = opts.output {
        std::fs::write(&path, &ws).expect("Cannot write output");
    } else {
        println!("{}", ws);
    }
}

fn load_module(path: &Path) -> Result<(String, String), Error> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.to_owned())
        .ok_or_else(|| Error::msg("Expected file name"))?;
    let content = fs::read_to_string(&path)?;
    Ok((name, content))
}
