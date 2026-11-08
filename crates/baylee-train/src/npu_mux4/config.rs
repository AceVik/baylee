//! Configuration, resource quotas, and scheduling urgency tiers for `npu_mux4`.

use std::path::PathBuf;

use anyhow::{bail, ensure};
use serde::{Deserialize, Serialize};

/// Scheduling urgency assigned to a session by the game coordinator.
///
/// The tiered pager evicts lower-urgency sessions (`Idle`, then `Soon`) to SSD
/// before higher-urgency sessions (`Immediate`), and broke ties by least-recently-used
/// monotonic tick.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SessionUrgency4 {
    /// Waiting on a slow human opponent, long timer, or between games: prime candidate for SSD swap.
    Idle = 0,
    /// Opponent is acting and priority is expected to return soon: keep in RAM if space permits.
    #[default]
    Soon = 1,
    /// Currently holds priority or is inside an active decision sequence: pinned in hot RAM.
    Immediate = 2,
}

/// Resource limits and security parameters for [`super::NpuMuxService4`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MuxConfig4 {
    /// Directory where encrypted cold-tier `.swap` files are stored (`0700` permissions).
    pub swap_dir: PathBuf,
    /// Number of resident neural worker slots kept warm in memory (`1..=256`).
    pub worker_slots: usize,
    /// Maximum number of session snapshots kept in Hot RAM before paging to SSD (`1..=65_536`).
    pub max_hot_sessions: usize,
    /// Soft ceiling on total Hot RAM snapshot bytes before paging out cold sessions.
    pub max_hot_ram_bytes: usize,
    /// Hard ceiling on total concurrent sessions (Hot RAM + Cold SSD) to prevent `DoS` exhaustion.
    pub max_total_sessions: usize,
    /// Maximum compressed + encrypted swap file size accepted from disk (bytes).
    pub max_swap_file_bytes: u64,
    /// Maximum uncompressed wire payload accepted when paging in from SSD (anti-zip-bomb guard).
    pub max_uncompressed_bytes: usize,
    /// Latent recurrent slot width `d_model` enforced on every snapshot load/store.
    pub d_model: usize,
}

impl MuxConfig4 {
    /// Creates a validated default configuration rooted at `swap_dir` for a model with `d_model`.
    #[must_use]
    pub fn new(swap_dir: PathBuf, d_model: usize) -> Self {
        Self {
            swap_dir,
            worker_slots: 4,
            max_hot_sessions: 64,
            max_hot_ram_bytes: 64 * 1024 * 1024,
            max_total_sessions: 4_096,
            max_swap_file_bytes: 512 * 1024,
            max_uncompressed_bytes: 2 * 1024 * 1024,
            d_model,
        }
    }

    /// Validates all quota invariants before service initialization.
    ///
    /// # Errors
    /// If any quota is zero, inverted, or exceeds safe bounds.
    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(
            (1..=256).contains(&self.worker_slots),
            "worker_slots ({}) must be in 1..=256",
            self.worker_slots
        );
        ensure!(
            (1..=65_536).contains(&self.max_hot_sessions),
            "max_hot_sessions ({}) must be in 1..=65_536",
            self.max_hot_sessions
        );
        ensure!(
            self.max_total_sessions >= self.max_hot_sessions,
            "max_total_sessions ({}) must be >= max_hot_sessions ({})",
            self.max_total_sessions,
            self.max_hot_sessions
        );
        ensure!(
            self.max_hot_ram_bytes >= 64 * 1024,
            "max_hot_ram_bytes ({}) must be at least 64 KiB",
            self.max_hot_ram_bytes
        );
        ensure!(
            (4_096..=16 * 1024 * 1024).contains(&self.max_swap_file_bytes),
            "max_swap_file_bytes ({}) out of bounds",
            self.max_swap_file_bytes
        );
        ensure!(
            (16 * 1024..=32 * 1024 * 1024).contains(&self.max_uncompressed_bytes),
            "max_uncompressed_bytes ({}) out of bounds",
            self.max_uncompressed_bytes
        );
        if self.d_model == 0 || self.d_model > 8_192 {
            bail!("d_model ({}) must be in 1..=8192", self.d_model);
        }
        Ok(())
    }
}
