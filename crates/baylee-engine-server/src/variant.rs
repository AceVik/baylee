//! The artifact's compile-time identity and its concrete hosting type.
use baylee_protocol::execution::{Identity, Variant};

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
pub type HostedSession = baylee_gamehost::Session<Detection, Registry>;

/// The registry compiled into the artifact's game and replay paths.
#[cfg(not(feature = "tower"))]
pub type Registry = baylee_gamehost::RegistryLookup;
/// The native Tower registry; ordinary setup is not admitted by this artifact.
#[cfg(feature = "tower")]
pub type Registry =
    baylee_engine::extensions::NativeLookup<baylee_gamehost::RegistryLookup, crate::tower::Pack>;

/// The concrete transport driver used by the executable.
pub type CompiledRunner = crate::EngineRunner<Detection, Registry>;

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
    #[cfg(feature = "tower")]
    let ai_generation = baylee_tower::v2::generation::CURRENT.ai_generation;
    #[cfg(not(feature = "tower"))]
    let ai_generation = baylee_core::preset::AIProfile::GENERATION;
    Identity::native(
        VARIANT,
        baylee_build::short(),
        baylee_build::COMMIT,
        u32::from(ai_generation),
    )
}

/// Checks the gateway's frozen setup against this executable before constructing a game.
#[must_use]
pub fn accepts(metadata: &[u8]) -> bool {
    Identity::from_metadata(metadata).as_ref() == Some(&identity())
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_engine::engine::LoopDetection;

    #[test]
    fn variant_metadata_and_compiled_policy_agree() {
        let identity = identity();
        assert!(identity.valid());
        assert!(accepts(&identity.to_metadata()));
        assert!(!accepts(b""));
        let mut wrong = identity.clone();
        wrong.features = "a".repeat(64);
        assert!(!accepts(&wrong.to_metadata()));
        assert!(Detection::supports(default_loop_policy()));
        assert_eq!(
            identity.variant == Variant::StandardLoops,
            cfg!(feature = "loop-detection")
        );
        assert_eq!(identity.variant == Variant::Tower, cfg!(feature = "tower"));
        #[cfg(feature = "tower")]
        assert_eq!(
            identity.rules,
            format!("tower-{}", baylee_tower::v2::REVISION)
        );
        if !cfg!(feature = "loop-detection") {
            assert_eq!(std::mem::size_of::<Detection>(), 0);
        }
    }
}
