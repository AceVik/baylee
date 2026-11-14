//! Engine profiling before the immutable deployment stage.
use super::{Deploy, Run, Stop, must, say, short};
use std::{fs, path::PathBuf};

pub(super) const BINARIES: [&str; 3] = [
    baylee_pgo::EngineArtifact::Standard.binary(),
    baylee_pgo::EngineArtifact::StandardLoops.binary(),
    baylee_pgo::EngineArtifact::Tower.binary(),
];

impl Deploy {
    /// Builds every static engine in isolation and proves each against its own
    /// plain build before making the complete set available to staging.
    ///
    /// Everything else stays as `stage` built it: the engine server is the one
    /// program here that plays the games a profile is made from, and the gateway
    /// and the agent do not link the engine at all (`docs/releasing.md`
    /// §"Profile-guided builds"). A step that fails stops the deploy, before
    /// anything is staged: an unprofiled engine does not go out silently.
    pub(super) fn profile_the_engine(&self, rev: &str) -> Run {
        must(&mut self.in_src(&[self.p.rustup.as_str(), "component", "add", "llvm-tools"]))?;
        let target = self.p.src.join("target");
        let profiles = target.join("pgo-profiles");
        // A profile belongs to one commit: none outlives the deploy it was for.
        let _ = fs::remove_dir_all(&profiles);
        let fail = |failure: baylee_pgo::Failure| {
            say(&format!("the profile-guided build stopped: {failure}"));
            Stop(1)
        };
        let plan = baylee_pgo::Plan {
            cargo: PathBuf::from(&self.p.cargo),
            profdata: baylee_pgo::profdata(self.p.rustc.as_ref()).map_err(fail)?,
            wrapper: std::env::current_exe().map_err(|_| Stop(1))?,
            root: self.p.src.clone(),
            work: profiles.join(rev),
            targets: target.join("pgo"),
            plain: target.clone(),
            profile: "release".to_owned(),
            ship: Vec::new(),
        };
        let mut built = Vec::new();
        for artifact in baylee_pgo::EngineArtifact::ALL {
            built.push((artifact, plan.for_artifact(artifact).build().map_err(fail)?));
        }
        // All three proofs passed. A failed variant never makes a partial
        // immutable stage, and the agent stays alive until the verified drain.
        for (artifact, directory) in built {
            fs::copy(
                directory.join("baylee-engine-server"),
                target.join("release").join(artifact.binary()),
            )
            .map_err(|error| {
                say(&format!("the profiled {} engine: {error}", artifact.name()));
                Stop(1)
            })?;
        }
        say(&format!(
            "all three engine variants {} profiled",
            short(rev)
        ));
        Ok(())
    }
}
