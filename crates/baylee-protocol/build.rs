//! Compiles the transport protobuf schema (protoc is vendored — no system
//! dependency required).

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    prost_build::Config::new()
        .protoc_executable(protoc_bin_vendored::protoc_bin_path()?)
        // The schema as protoc read it, for `tests/engine_link.rs`, which
        // holds it to everything it already said.
        .file_descriptor_set_path(out.join("transport.fds"))
        // A seat or spectator frame's payload is handed on, never read, by the
        // gateway: as `Bytes` it is a slice of the frame it arrived in, and
        // handing it to a socket is a reference count rather than a copy.
        .bytes([
            ".baylee.v1.SeatFrame.envelope",
            ".baylee.v1.SpectatorFrame.envelope",
        ])
        .compile_protos(&["proto/baylee/v1/transport.proto"], &["proto"])?;
    println!("cargo:rerun-if-changed=proto/baylee/v1/transport.proto");
    Ok(())
}
