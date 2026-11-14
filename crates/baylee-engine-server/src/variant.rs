//! The artifact's compile-time identity and its concrete hosting type.
use baylee_protocol::execution::{Identity, SCHEMA, Variant};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

#[cfg(all(feature = "tower", feature = "loop-detection"))]
compile_error!(
    "Tower must be built without loop-detection; use --no-default-features --features tower"
);

/// The detector compiled into this executable's game path.
#[cfg(feature = "loop-detection")]
pub type Detection = baylee_engine::engine::DetectLoops;
/// No detector state or detector work in this executable's game path.
#[cfg(not(feature = "loop-detection"))]
pub type Detection = baylee_engine::engine::NoLoopDetection;

/// The concrete session, including its recorder and house-AI simulation policy.
pub type HostedSession = baylee_gamehost::Session<Detection>;

/// The concrete transport driver used by the executable.
pub type CompiledRunner = crate::EngineRunner<Detection>;

/// This executable's variant; no environment variable can change it.
pub const VARIANT: Variant = if cfg!(feature = "tower") {
    Variant::Tower
} else if cfg!(feature = "loop-detection") {
    Variant::StandardLoops
} else {
    Variant::Standard
};

/// The preset policy of a fresh isolated development table for this artifact.
#[must_use]
pub const fn default_loop_policy() -> baylee_core::preset::LoopPolicy {
    if cfg!(feature = "loop-detection") {
        baylee_core::preset::LoopPolicy::RunOnceThenBreak
    } else {
        baylee_core::preset::LoopPolicy::Off
    }
}

/// Emits compatibility metadata without opening a socket or reading credentials.
#[must_use]
pub fn identity() -> Identity {
    let features = match VARIANT {
        Variant::Standard => "kernel=2;loops=off;pack=standard;fork=on",
        Variant::StandardLoops => "kernel=2;loops=on;pack=standard;fork=on",
        Variant::Tower => "kernel=2;loops=off;pack=tower;fork=on",
    };
    let pack = if VARIANT == Variant::Tower {
        "tower"
    } else {
        "standard"
    };
    let mut fingerprint = String::with_capacity(64);
    for byte in Sha256::digest(features.as_bytes()) {
        write!(&mut fingerprint, "{byte:02x}").expect("writing into a string");
    }
    Identity {
        schema: SCHEMA,
        protocol: baylee_protocol::PROTOCOL_VERSION,
        variant: VARIANT,
        build: baylee_build::short().into(),
        features: fingerprint,
        rules: format!("{pack}-2"),
        cards: format!("registry@{}", baylee_build::COMMIT),
        ai: format!(
            "{pack}-house-{}",
            baylee_core::preset::AIProfile::GENERATION
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_engine::engine::LoopDetection;

    #[test]
    fn variant_metadata_and_compiled_policy_agree() {
        let identity = identity();
        assert!(identity.valid());
        assert!(Detection::supports(default_loop_policy()));
        assert_eq!(
            identity.variant == Variant::StandardLoops,
            cfg!(feature = "loop-detection")
        );
        assert_eq!(identity.variant == Variant::Tower, cfg!(feature = "tower"));
        if !cfg!(feature = "loop-detection") {
            assert_eq!(std::mem::size_of::<Detection>(), 0);
        }
    }
}
