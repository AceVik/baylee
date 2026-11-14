//! Engine profiling before the immutable deployment stage.
use super::{Deploy, Run, Stop, must, say, short};
use std::{fs, path::PathBuf};

impl Deploy {
    /// Replaces the plain build's engine server with one built from a profile
    /// of this commit, proven to play the plain build's games (`baylee_pgo`).
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
            ship: [
                "-p",
                "baylee-engine-server",
                "--bin",
                "baylee-engine-server",
            ]
            .map(str::to_owned)
            .to_vec(),
        };
        let built = plan.build().map_err(fail)?;
        fs::copy(
            built.join("baylee-engine-server"),
            target.join("release/baylee-engine-server"),
        )
        .map_err(|error| {
            say(&format!("the profiled engine server: {error}"));
            Stop(1)
        })?;
        say(&format!("engine server {} profiled", short(rev)));
        Ok(())
    }
}
