// see https://doc.rust-lang.org/cargo/reference/build-scripts.html
extern crate tonic_build;

use std::path::Path;

const OUT_DIR: &str = "src/compiled_protos";
const PB_PATH: [&str; 2] = [
    "vm-proto/protos/vm.proto",
    "vm-proto/protos/data-source.proto",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/build.rs");
    println!("cargo:rerun-if-changed=Cargo.lock");

    for path in PB_PATH.iter() {
        println!("rerun-if-changed={}", path);
        println!("cargo:rerun-if-changed={}", path);
        let proto_path: &Path = path.as_ref();
        let proto_dir = proto_path
            .parent()
            .expect("proto file should reside in a directory");
        tonic_build::configure()
            .out_dir(OUT_DIR)
            .compile(&[proto_path], &[proto_dir])?
    }

    Ok(())
}
