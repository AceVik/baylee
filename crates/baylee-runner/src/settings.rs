//! Runner configuration and its environment compatibility surface.

use std::path::PathBuf;
use std::time::Duration;

/// The runner's settings, with the shell runner's names and bounds.
#[derive(Clone, Debug)]
pub struct Settings {
    /// GitHub repository in `owner/name` form.
    pub repo: String,
    /// Local Tart image cloned for each job.
    pub image: String,
    /// Upstream image used by `prepare`, pinned by digest: the macOS archives
    /// a release signs are built in it, and a moving tag would change them
    /// unseen. Raise it with the actions-runner pin.
    pub base: String,
    /// The repository-wide Cargo lock command.
    pub lock: PathBuf,
    /// Keychain service holding the fine-grained token.
    pub keychain_service: String,
    /// GitHub label requested by package jobs.
    pub label: String,
    /// Delay between queue polls.
    pub poll: Duration,
    /// Idle registration allowance before it is stopped.
    pub idle: Duration,
    /// Absolute lifetime of one job VM.
    pub job_cap: Duration,
    /// Jobs run side by side, at most the two VMs macOS allows. Each VM starts
    /// once the one before it reached the network: two started together both
    /// stayed offline for twelve minutes (29.10.2026). One by default since
    /// 30.10.2026 (owner), though two are faster: both packages took 43–44
    /// minutes side by side against 70 one after the other
    /// (`docs/releasing.md` §"The macOS runner").
    pub max_vms: usize,
    /// vCPUs assigned by `prepare`.
    pub vm_cpus: usize,
    /// Memory assigned by `prepare`, in MiB.
    pub vm_memory_mb: usize,
    /// Disk assigned by `prepare`, in GiB.
    pub vm_disk_gb: usize,
    /// Pinned actions/runner release.
    pub runner_version: String,
    /// SHA-256 of that release's archive.
    pub runner_sha256: String,
}

impl Settings {
    /// Reads the four optional overrides `BAYLEE_RUNNER_REPO`,
    /// `BAYLEE_RUNNER_IMAGE`, `BAYLEE_RUNNER_BASE` and `BAYLEE_CARGO_LOCK`.
    #[must_use]
    pub fn from_env() -> Self {
        let mut settings = Self::default();
        if let Ok(value) = std::env::var("BAYLEE_RUNNER_REPO") {
            settings.repo = value;
        }
        if let Ok(value) = std::env::var("BAYLEE_RUNNER_IMAGE") {
            settings.image = value;
        }
        if let Ok(value) = std::env::var("BAYLEE_RUNNER_BASE") {
            settings.base = value;
        }
        if let Some(value) = std::env::var_os("BAYLEE_CARGO_LOCK") {
            settings.lock = PathBuf::from(value);
        }
        settings
    }

    /// A configuration with zero-time bounds for immediate test decisions.
    #[cfg(test)]
    #[must_use]
    pub fn for_tests() -> Self {
        Self {
            idle: Duration::ZERO,
            job_cap: Duration::ZERO,
            ..Self::default()
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var_os("HOME").unwrap_or_default();
        Self {
            repo: "AceVik/baylee".to_owned(),
            image: "baylee-runner".to_owned(),
            base: "ghcr.io/cirruslabs/macos-tahoe-base@sha256:4aaa751fa7538902d73c767f27a066fc87e83f183069da4e4b5022863abd4c77"
                .to_owned(),
            lock: PathBuf::from(home).join(".baylee-locks/with-cargo-lock.sh"),
            keychain_service: "baylee-macos-runner".to_owned(),
            label: "baylee-vm".to_owned(),
            poll: Duration::from_secs(60),
            idle: Duration::from_mins(15),
            job_cap: Duration::from_secs(10_800),
            max_vms: 1,
            vm_cpus: 8,
            vm_memory_mb: 16_384,
            vm_disk_gb: 120,
            runner_version: "2.338.0".to_owned(),
            runner_sha256: "df4cebda25c86a886ed204e49fee63f5c2e7cec5f447b5c98440a826bbdf9df2"
                .to_owned(),
        }
    }
}
