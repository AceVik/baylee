//! Each engine variant's selection, as cargo resolves it: the features of
//! `baylee-engine-server` that one build of the variant turns on, together
//! with the workload built in the same run (`baylee-perf`). The standard-loops
//! selection once named `baylee-engine-server/loop-detection`, which also
//! switched on the workload's optional engine dependency and, with it, Tower,
//! and the engine refused to compile (27.11.2026, found by rehearsing a deploy).
use baylee_pgo::EngineArtifact;
use std::{path::Path, process::Command};

/// The engine server's features in one variant's selection.
fn engine_features(artifact: EngineArtifact) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut args = artifact.cargo_args();
    // `cargo tree` selects packages, not binaries.
    while let Some(at) = args.iter().position(|arg| arg == "--bin") {
        args.drain(at..=at + 1);
    }
    let output = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(&root)
        .args([
            "tree",
            "--locked",
            "--offline",
            "-e",
            "features",
            "-p",
            "baylee-perf",
        ])
        .args(&args)
        .args(["-i", "baylee-engine-server", "--prefix", "none"])
        .output()
        .expect("cargo tree runs");
    assert!(
        output.status.success(),
        "cargo tree for {}: {}",
        artifact.name(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn each_variant_turns_on_exactly_its_own_engine_features() {
    for artifact in EngineArtifact::ALL {
        let tree = engine_features(artifact);
        let on =
            |feature: &str| tree.contains(&format!("baylee-engine-server feature \"{feature}\""));
        let (loops, tower) = match artifact {
            EngineArtifact::Standard => (false, false),
            EngineArtifact::StandardLoops => (true, false),
            EngineArtifact::Tower => (false, true),
        };
        assert_eq!(
            on("loop-detection"),
            loops,
            "{}: loop detection\n{tree}",
            artifact.name()
        );
        assert_eq!(on("tower"), tower, "{}: Tower\n{tree}", artifact.name());
    }
}
