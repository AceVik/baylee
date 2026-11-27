//! Engine workload profiling and its semantic comparison.
use super::{
    FLAG_SEPARATOR, FLAGS, Failure, MOST_UNMATCHED_PERCENT, PACKAGE, PROVEN_ON, REPORT_UNMATCHED,
    TRAINED_ON, UNMATCHED, WORKLOAD, output_of, run,
};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

mod artifact;
mod shapes;
pub use artifact::EngineArtifact;

/// One profile-guided build.
#[derive(Clone, Debug)]
pub struct Plan {
    /// The `cargo` to run.
    pub cargo: PathBuf,
    /// The `llvm-profdata` of this toolchain ([`crate::profdata`]).
    pub profdata: PathBuf,
    /// A program that calls [`crate::wrap_rustc`] first: cargo starts it in place of
    /// rustc for the workspace's crates.
    pub wrapper: PathBuf,
    /// The checkout, at the commit being built.
    pub root: PathBuf,
    /// Where this commit's counts and profile go.
    pub work: PathBuf,
    /// Where the instrumented and the profiled builds keep their target
    /// directories. Emptied first: see the crate's comment.
    pub targets: PathBuf,
    /// The plain build's target directory.
    pub plain: PathBuf,
    /// The cargo profile, `release` or `dist`.
    pub profile: String,
    /// The `cargo build` arguments naming what ships, `-p baylee-engine-server`.
    pub ship: Vec<String>,
}

impl Plan {
    /// Gives an artifact its own counts, target directories and plain proof.
    /// Nothing is shared with another variant through Cargo feature unification.
    #[must_use]
    pub fn for_artifact(&self, artifact: EngineArtifact) -> Self {
        Self {
            work: self.work.join(artifact.name()),
            targets: self.targets.join(artifact.name()),
            plain: self.plain.join("pgo-plain").join(artifact.name()),
            ship: artifact.cargo_args(),
            ..self.clone()
        }
    }

    /// Trains, builds and proves; the directory of the profiled binaries.
    ///
    /// # Errors
    /// [`Failure`] at the first step that fails, when the profiled build plays
    /// anything differently from the plain one, or when the profile took no
    /// effect.
    pub fn build(&self) -> Result<PathBuf, Failure> {
        let shapes = shapes::Shapes::for_args(&self.ship)?;
        let raw = self.work.join("raw");
        let _ = fs::remove_dir_all(&raw);
        let _ = fs::remove_dir_all(&self.targets);
        fs::create_dir_all(&raw).map_err(|e| Failure(format!("{}: {e}", raw.display())))?;

        // The same selection as the profiled build below, or the counts match
        // nothing: a function is found by its mangled name, which holds a hash
        // of its crate's features, and what a `cargo` run selects unifies them.
        let generate = self.targets.join("generate");
        let flag = format!("-Cprofile-generate={}", raw.display());
        self.cargo(&generate, &[&flag], &self.ship)?;
        for shape in shapes.trained {
            run(
                Command::new(self.workload(&generate))
                    .arg(shape)
                    .current_dir(&self.root)
                    .env("LLVM_PROFILE_FILE", raw.join("%m_%p.profraw")),
                &format!("training on {shape}"),
            )?;
        }
        let merged = self.work.join("merged.profdata");
        run(
            Command::new(&self.profdata)
                .arg("merge")
                .arg("-o")
                .arg(&merged)
                .arg(&raw),
            "merging the counts",
        )?;

        let profiled = self.targets.join("profiled");
        let flag = format!("-Cprofile-use={}", merged.display());
        let report = self.cargo(&profiled, &[&flag, REPORT_UNMATCHED], &self.ship)?;
        self.require_counts_used(&merged, &report)?;
        // The plain proof must instantiate the same static engine as training
        // and shipping. Dropping `ship` here silently tests a different variant.
        self.cargo(&self.plain, &[], &self.ship)?;
        for shape in shapes.proven {
            let plain = self.verify(&self.plain, shape)?;
            let guided = self.verify(&profiled, shape)?;
            if plain != guided {
                return Err(Failure(format!(
                    "the profiled build does not play {shape} as the plain build does"
                )));
            }
        }
        Ok(self.out(&profiled))
    }

    /// Holds the compiler's `report` of the profiled build to the profile.
    fn require_counts_used(&self, merged: &Path, report: &str) -> Result<(), Failure> {
        let output = Command::new(&self.profdata)
            .arg("show")
            .arg(merged)
            .output()
            .map_err(|e| Failure(format!("reading the profile: {e}")))?;
        let shown = String::from_utf8_lossy(&output.stdout);
        let total: usize = shown
            .lines()
            .find_map(|line| line.strip_prefix("Total functions: "))
            .and_then(|count| count.trim().parse().ok())
            .ok_or_else(|| Failure(format!("the profile names no function count: {shown}")))?;
        let unmatched = report.matches(UNMATCHED).count();
        if total == 0 || unmatched * 100 > total * MOST_UNMATCHED_PERCENT {
            return Err(Failure(format!(
                "the profile took no effect: the compiler found no counts for {unmatched} functions \
                 against {total} in the profile, and {MOST_UNMATCHED_PERCENT} % is the most allowed"
            )));
        }
        Ok(())
    }

    /// Where a target directory's binaries are.
    fn out(&self, target: &Path) -> PathBuf {
        target.join(if self.profile == "dev" {
            "debug"
        } else {
            &self.profile
        })
    }

    fn workload(&self, target: &Path) -> PathBuf {
        self.out(target)
            .join(format!("{WORKLOAD}{}", env::consts::EXE_SUFFIX))
    }

    /// Builds the workload and `ship` in `target`, with `flags` on the
    /// workspace's crates, and answers what the compiler said.
    fn cargo(&self, target: &Path, flags: &[&str], ship: &[String]) -> Result<String, Failure> {
        let what = format!("building in {}", target.display());
        let mut build = Command::new(&self.cargo);
        build
            .args(["build", "--locked", "--profile", &self.profile])
            .args(["-p", PACKAGE, "--bin", WORKLOAD])
            .args(ship)
            .current_dir(&self.root)
            .env("CARGO_TARGET_DIR", target);
        if !flags.is_empty() {
            build
                .env("RUSTC_WORKSPACE_WRAPPER", &self.wrapper)
                .env(FLAGS, flags.join(&FLAG_SEPARATOR.to_string()));
        }
        output_of(&mut build, &what)
    }

    fn verify(&self, target: &Path, shape: &str) -> Result<Vec<u8>, Failure> {
        let what = format!("verifying {shape} in {}", target.display());
        let output = Command::new(self.workload(target))
            .args([shape, "--verify"])
            .current_dir(&self.root)
            .output()
            .map_err(|e| Failure(format!("{what}: {e}")))?;
        if output.status.success() {
            Ok(output.stdout)
        } else {
            Err(Failure(format!("{what}: {}", output.status)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifacts_never_share_counts_builds_or_plain_proofs() {
        let plan = Plan {
            cargo: "cargo".into(),
            profdata: "llvm-profdata".into(),
            wrapper: "wrapper".into(),
            root: "checkout".into(),
            work: "profiles".into(),
            targets: "target/pgo".into(),
            plain: "target".into(),
            profile: "release".into(),
            ship: Vec::new(),
        };
        let plans = EngineArtifact::ALL.map(|artifact| plan.for_artifact(artifact));
        for (at, chosen) in plans.iter().enumerate() {
            assert!(chosen.ship.contains(&"--no-default-features".to_owned()));
            for other in &plans[..at] {
                assert_ne!(chosen.work, other.work);
                assert_ne!(chosen.targets, other.targets);
                assert_ne!(chosen.plain, other.plain);
                assert_ne!(chosen.ship, other.ship);
            }
        }
        assert_eq!(
            EngineArtifact::named("standard"),
            Some(EngineArtifact::Standard)
        );
        assert_eq!(
            EngineArtifact::named("standard-loops"),
            Some(EngineArtifact::StandardLoops)
        );
        assert_eq!(EngineArtifact::named("tower"), Some(EngineArtifact::Tower));
        for bad in ["", "Tower", "../standard", "standard --features tower"] {
            assert_eq!(EngineArtifact::named(bad), None);
        }
    }
}
