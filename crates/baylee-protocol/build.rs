//! Compiles the transport protobuf schema (protoc is vendored — no system
//! dependency required).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        .compile_protos(&["proto/baylee/v1/transport.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/baylee/v1/transport.proto");
    Ok(())
}
