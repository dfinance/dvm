#[macro_use]
extern crate structopt;

use structopt::StructOpt;
use http::Uri;
use std::env;
use dvm_compiler::{
    manifest::{MANIFEST, CmoveToml},
    cmd::*,
};
use std::process::exit;
use std::path::Path;
use anyhow::Error;
use dvm_compiler::manifest::read_manifest;

#[derive(StructOpt, Debug)]
#[structopt(name = "git")]
enum Opt {
    #[structopt(help = "Init directory as cmove project.")]
    Init {
        #[structopt(help = "Project name.")]
        project_name: String,
        #[structopt(
            help = "Basic uri to blockchain api.",
            name = "Blockchain API",
            long = "repo",
            short = "r"
        )]
        repository: Option<Uri>,
        #[structopt(
            help = "Account address.",
            name = "address",
            long = "address",
            short = "a"
        )]
        address: Option<String>,
    },
    #[structopt(help = "Create a new cmove project")]
    New {
        #[structopt(help = "Project name.")]
        project_name: String,
        #[structopt(
            help = "Basic uri to blockchain api.",
            name = "Blockchain API",
            long = "repo",
            short = "r"
        )]
        repository: Option<Uri>,
        #[structopt(
            help = "Account address.",
            name = "address",
            long = "address",
            short = "a"
        )]
        address: Option<String>,
    },
    #[structopt(help = "Reload dependencies")]
    Update {},
    #[structopt(help = "Build project")]
    Build {},
}

fn main() {
    let project_dir = env::current_dir().unwrap();
    let matches = Opt::from_args();
    handle_error(match matches {
        Opt::New {
            project_name: source_dir,
            repository,
            address,
        } => new::execute(&project_dir, source_dir, repository, address),
        Opt::Init {
            project_name: source_dir,
            repository,
            address,
        } => init::execute(&project_dir, source_dir, repository, address),
        Opt::Update {} => update::execute(&project_dir, load_manifest(&project_dir)),
        Opt::Build {} => build::execute(&project_dir, load_manifest(&project_dir)),
    });
}

fn handle_error<T>(res: Result<T, Error>) -> T {
    match res {
        Ok(t) => t,
        Err(err) => {
            println!("error: {:?}.", err);
            exit(1);
        }
    }
}

fn load_manifest(project_dir: &Path) -> CmoveToml {
    let manifest = project_dir.join(MANIFEST);
    if !manifest.exists() {
        println!(
            "error: could not find `{}` in `{:?}`.",
            MANIFEST, project_dir
        );
        exit(1);
    }
    match read_manifest(&manifest) {
        Ok(manifest) => manifest,
        Err(_) => {
            println!("error: could not read `{:?}`.", &manifest);
            exit(1);
        }
    }
}
