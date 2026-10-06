//! Compiles the transport protobuf schema (protoc is vendored — no system
//! dependency required).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        // A seat frame's payload is handed on, never read, by the gateway:
        // as `Bytes` it is a slice of the frame it arrived in, and handing
        // it to a seat's socket is a reference count rather than a copy.
        .bytes([".baylee.v1.SeatFrame.envelope"])
        .compile_protos(&["proto/baylee/v1/transport.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/baylee/v1/transport.proto");
    Ok(())
}
