//! Records the screens the framework's unit tests read (`tests/fixtures/`)
//! from a real client, for when `/state` changes shape:
//!
//! ```bash
//! cargo build -p baylee-client --features dev-control
//! cargo run -p baylee-client-e2e --example record_fixtures
//! ```
//!
//! An example and not a test, so `cargo dev e2e` never rewrites them. The
//! client plays offline and signs in nowhere, so a recording holds no secret.
use baylee_client_e2e::prelude::*;
use std::{fs, path::Path};

fn record(name: &str, screen: &Screen) -> Result<()> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(format!("{name}.json"));
    fs::write(&path, serde_json::to_vec_pretty(&screen.recording())?)?;
    println!("wrote {}", path.display());
    Ok(())
}

fn main() {
    e2e::run(|client| {
        record("front-laptop", &client.settled()?)?;
        record("front-phone", &client.resize(Viewport::PHONE)?)?;
        client.resize(Viewport::LAPTOP)?;
        record("duel-mulligan", &client.play_the_house()?)
    });
}
