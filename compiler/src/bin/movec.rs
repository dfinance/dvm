extern crate clap;

use clap::Clap;
use http::Uri;
use std::env;
use dvm_compiler::{
    manifest::{MANIFEST, MoveToml},
    cmd::*,
};
use std::process::exit;
use std::path::Path;
use anyhow::Error;
use dvm_compiler::manifest::read_manifest;

#[derive(Clap, Debug)]
#[clap(name = "Move compiler.")]
enum Opt {
    #[clap(about = "Init directory as move project.")]
    Init {
        /// Project name.
        project_name: String,
        #[clap(name = "Blockchain API", long = "repo", short = 'r')]
        /// Basic uri to blockchain api.
        repository: Option<Uri>,
        #[clap(name = "address", long = "address", short = 'a')]
        /// Account address.
        address: Option<String>,
    },
    #[clap(about = "Create a new move project")]
    New {
        /// Project name.
        project_name: String,
        #[clap(name = "Blockchain API", long = "repo", short = 'r')]
        /// Basic uri to blockchain api.
        repository: Option<Uri>,
        #[clap(name = "address", long = "address", short = 'a')]
        /// Account address.
        address: Option<String>,
    },
    #[clap(about = "Reload dependencies")]
    Update {},
    #[clap(about = "Build project")]
    Build {},
    #[clap(about = "Check project")]
    Check {},
}

fn main() {
    let project_dir = env::current_dir().unwrap();
    let matches = Opt::parse();
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
        Opt::Check {} => check::execute(&project_dir, load_manifest(&project_dir)),
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

fn load_manifest(project_dir: &Path) -> MoveToml {
    let manifest = project_dir.join(MANIFEST);
    if !manifest.exists() {
        println!(
            "error: could not find `{}` in `{:?}`.",
            MANIFEST, project_dir
        );
        exit(1);
    }
    match read_manifest(&manifest) {
        Ok(mut manifest) => {
            if manifest.layout.is_none() {
                manifest.layout = Some(Default::default());
            }
            if let Some(layout) = manifest.layout.as_mut() {
                layout.fill();
            }
            manifest
        }
        Err(_) => {
            println!("error: could not read `{:?}`.", &manifest);
            exit(1);
        }
    }
}
