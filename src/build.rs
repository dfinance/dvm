// see https://doc.rust-lang.org/cargo/reference/build-scripts.html

extern crate tonic_build;

const PB_PATH: [&'static str; 2] = [
    "vm-proto/protos/vm.proto",
    "vm-proto/protos/data-source.proto",
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/build.rs");
    println!("cargo:rerun-if-changed=Cargo.lock");

    for path in PB_PATH.iter() {
        println!("rerun-if-changed={}", path);
        println!("cargo:rerun-if-changed={}", path);
        tonic_build::compile_protos(path)?;
    }

    Ok(())
}
