use std::{fs, io, path::PathBuf};
use structopt::StructOpt;
use serde_json::{to_string, to_string_pretty};
use dvm::vm::stdlib::{Stdlib, build_std, WS};
use dvm::vm::Lang;

#[derive(StructOpt)]
struct Opts {
    /// Path to the directory with the standard library.
    #[structopt()]
    source_dir: String,
    #[structopt(help = "Compiler type; [move, mvir]")]
    lang: Lang,
    /// Optional path to the output file.
    /// If not passed, result will be printed to stdout.
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,

    #[structopt(short = "v", long = "verbose")]
    /// Verbose mode flag.
    /// Enables debug printing of internals including used modules.
    debug_print: bool,
    /// Enables pretty printing of all output including debug-prints if it enabled.
    #[structopt(short)]
    pretty_print: bool,
}

fn main() {
    let opts = Opts::from_args();

    let extension = match &opts.lang {
        Lang::MvIr => "mvir",
        Lang::Move => "move",
    };

    let entries = fs::read_dir(&opts.source_dir)
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .filter(|path| {
            if match path {
                Ok(path) => path
                    .extension()
                    .map(|ext| ext == extension)
                    .unwrap_or(false),
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

    let modules: Vec<_> = entries
        .iter()
        .map(fs::read_to_string)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let std = Stdlib {
        modules: modules.iter().map(|s| s.as_str()).collect(),
        lang: opts.lang,
    };

    // Build
    let vm_value = build_std(std).unwrap();

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
