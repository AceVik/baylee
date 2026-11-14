//! Separate Cargo invocations prevent feature unification between engine variants.

/// One executable to build, profile and install. These are local operator choices,
/// never arbitrary build arguments or paths received from a gateway.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineArtifact {
    /// Ordinary Magic, with no loop detector compiled into its session.
    Standard,
    /// Ordinary Magic, with loop detection available.
    StandardLoops,
    /// The native Tower pack, always without loop detection.
    Tower,
}

impl EngineArtifact {
    /// The complete installed set, each built in its own Cargo invocation.
    pub const ALL: [Self; 3] = [Self::Standard, Self::StandardLoops, Self::Tower];

    /// The protocol variant and the subdirectory of its own counts and build.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::StandardLoops => "standard-loops",
            Self::Tower => "tower",
        }
    }

    /// Installed executable's stem. The standard artifact keeps the original
    /// name so existing operator configuration still locates the primary engine.
    #[must_use]
    pub const fn binary(self) -> &'static str {
        match self {
            Self::Standard => "baylee-engine-server",
            Self::StandardLoops => "baylee-engine-standard-loops",
            Self::Tower => "baylee-engine-tower",
        }
    }

    /// The same package/feature selection for training, shipping and the plain
    /// semantic proof. The workload must instantiate the shipping detector type.
    #[must_use]
    pub fn cargo_args(self) -> Vec<String> {
        let features = match self {
            Self::Standard => "baylee-perf/without-loop-detection",
            Self::StandardLoops => "baylee-engine-server/loop-detection",
            Self::Tower => "baylee-engine-server/tower,baylee-perf/without-loop-detection",
        };
        [
            "-p",
            "baylee-engine-server",
            "--bin",
            "baylee-engine-server",
            "--no-default-features",
            "--features",
            features,
        ]
        .map(str::to_owned)
        .to_vec()
    }

    /// Parses an exact operator selection; unknown names never fall back.
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|artifact| artifact.name() == name)
    }
}
