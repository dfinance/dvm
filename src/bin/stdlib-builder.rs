use structopt::StructOpt;

use std::{fs, io};
use move_vm_in_cosmos::vm::stdlib::{Stdlib, build_std, WS};
use move_vm_in_cosmos::vm::Lang;

#[derive(StructOpt)]
struct Opts {
    #[structopt(help = "Path to the directory with the standard library")]
    source_dir: String,
    #[structopt(help = "Compiler type; [move, mvir]")]
    lang: Lang,
}

fn main() {
    let Opts { source_dir, lang } = Opts::from_args();

    let extension = match lang {
        Lang::MvIr => "mvir",
        Lang::Move => "move",
    };

    let entries = fs::read_dir(source_dir)
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

    println!(
        "Modules:{:?}",
        entries
            .iter()
            .map(|p| p.file_name().unwrap())
            .collect::<Vec<_>>()
    );

    let modules = entries
        .iter()
        .map(fs::read_to_string)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let std = Stdlib {
        modules: modules.iter().map(|s| s.as_str()).collect(),
        lang,
    };

    let vm_value = build_std(std).unwrap();

    let ws = serde_json::to_string_pretty(&WS::from(vm_value)).unwrap();
    println!("{}", ws);
}
