// see https://doc.rust-lang.org/cargo/reference/build-scripts.html

extern crate tonic_build;

const PB_PATH: &str = "protobuf/vm.proto";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/build.rs");
    println!("cargo:rerun-if-changed=Cargo.lock");

    println!("rerun-if-changed={}", PB_PATH);
    println!("cargo:rerun-if-changed={}", PB_PATH);

    tonic_build::compile_protos(PB_PATH)?;

    Ok(())
}
