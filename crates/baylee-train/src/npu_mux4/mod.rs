//! NPU Session-Multiplexer & Tiered RAM/SSD Session Pager (`npu_mux4`),
//! backed by the multi-hardware [`baylee_driver`] runtime crate.
//!
//! Re-exports the generic [`baylee_driver`] types specialized to [`NetPlayer4`]
//! and [`PlayerSnapshot4`] and provides `NetPlayer4` model-loading constructors.

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::ensure;

pub use baylee_driver::config::{MuxConfig4, SessionUrgency4};
pub use baylee_driver::hardware::{AcceleratorKind, GraphPrecision, HardwareConfig};
pub use baylee_driver::metrics::{MuxMetrics4, MuxSummary4};
pub use baylee_driver::mux::{ActWaveItem4, ObserveWaveItem4};
pub use baylee_driver::swap_store::EncryptedSwapStore4;
pub use baylee_driver::worker_pool::WaveSlotOutput4;

use crate::netplay4::{
    BatchLimits4, BatchedNet4, CatalogPolicy4, ColdMeta4, NetPlayer4, PlayerSnapshot4, PolicyMode4,
};

/// Cold SSD swap handle specialized to [`ColdMeta4`].
pub type SwapHandle4 = baylee_driver::swap_store::SwapHandle4<ColdMeta4>;

/// Tiered Hot-RAM / Cold-SSD session pool specialized to [`PlayerSnapshot4`].
pub type TieredSessionPool4 = baylee_driver::scheduler::TieredSessionPool4<PlayerSnapshot4>;

/// Fixed pool of warm neural worker slots specialized to [`NetPlayer4`].
pub type NpuWorkerPool4 = baylee_driver::worker_pool::NpuWorkerPool4<NetPlayer4>;

/// High-level NPU/GPU/CPU session multiplexer specialized to [`NetPlayer4`].
pub type NpuMuxService4 = baylee_driver::mux::NpuMuxService4<NetPlayer4>;

/// Extension constructors for [`NpuWorkerPool4`] backed by [`BatchedNet4`] and [`NetPlayer4`].
pub trait NetWorkerPoolExt: Sized {
    /// Loads a single shared [`BatchedNet4`] model into memory and spawns `worker_slots`
    /// lightweight [`NetPlayer4`] worker slots attached to it.
    ///
    /// # Errors
    /// If the model cannot be opened or `worker_slots == 0`.
    fn load_shared(
        model_path: &Path,
        worker_slots: usize,
        batch_limits: BatchLimits4,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
    ) -> anyhow::Result<Self>;

    /// Loads a single shared [`BatchedNet4`] model configured for `hardware` and spawns
    /// `worker_slots` [`NetPlayer4`] worker slots attached to it.
    ///
    /// # Errors
    /// If the model cannot be opened or `worker_slots == 0`.
    fn load_shared_with_hardware(
        model_path: &Path,
        worker_slots: usize,
        batch_limits: BatchLimits4,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
        hardware: &HardwareConfig,
    ) -> anyhow::Result<Self>;

    /// Builds `worker_slots` workers over an already-opened [`BatchedNet4`].
    ///
    /// # Errors
    /// If `worker_slots == 0`.
    fn from_shared_net(
        shared_net: &Arc<BatchedNet4>,
        worker_slots: usize,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
    ) -> anyhow::Result<Self>;
}

impl NetWorkerPoolExt for NpuWorkerPool4 {
    fn load_shared(
        model_path: &Path,
        worker_slots: usize,
        batch_limits: BatchLimits4,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
    ) -> anyhow::Result<Self> {
        let hw = HardwareConfig::default();
        Self::load_shared_with_hardware(
            model_path,
            worker_slots,
            batch_limits,
            policy_mode,
            temperature,
            thinking_steps,
            &hw,
        )
    }

    fn load_shared_with_hardware(
        model_path: &Path,
        worker_slots: usize,
        batch_limits: BatchLimits4,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
        hardware: &HardwareConfig,
    ) -> anyhow::Result<Self> {
        ensure!(worker_slots > 0, "worker_slots must be > 0");
        let shared_net = BatchedNet4::load_with_hardware(
            model_path,
            batch_limits,
            CatalogPolicy4::Superset,
            hardware,
            worker_slots,
        )?;
        Self::from_shared_net(
            &shared_net,
            worker_slots,
            policy_mode,
            temperature,
            thinking_steps,
        )
    }

    fn from_shared_net(
        shared_net: &Arc<BatchedNet4>,
        worker_slots: usize,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
    ) -> anyhow::Result<Self> {
        ensure!(worker_slots > 0, "worker_slots must be > 0");
        let mut workers = Vec::with_capacity(worker_slots);
        for _ in 0..worker_slots {
            let mut player = NetPlayer4::shared(shared_net)
                .with_policy_mode(policy_mode)
                .with_temperature(temperature);
            if let Some(k) = thinking_steps {
                player.set_thinking_steps(k);
            }
            workers.push(player);
        }
        Self::from_workers(workers)
    }
}

/// Extension constructors for [`NpuMuxService4`] backed by [`NetPlayer4`].
pub trait NetMuxServiceExt: Sized {
    /// Opens a model once and initializes `worker_slots` shared workers plus
    /// an encrypted SSD swap directory at `swap_dir`.
    ///
    /// # Errors
    /// If the model cannot be loaded or the swap directory / quotas are invalid.
    #[allow(clippy::too_many_arguments, reason = "service builder parameters")]
    fn open(
        model_path: &Path,
        swap_dir: PathBuf,
        worker_slots: usize,
        max_hot_sessions: usize,
        max_hot_ram_bytes: usize,
        batch_limits: BatchLimits4,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
    ) -> anyhow::Result<Self>;

    /// Opens a model configured with [`HardwareConfig`] and initializes `worker_slots`
    /// shared workers plus an encrypted SSD swap directory at `swap_dir`.
    ///
    /// # Errors
    /// If the model cannot be loaded or the swap directory / quotas are invalid.
    #[allow(clippy::too_many_arguments, reason = "service builder parameters")]
    fn open_with_hardware(
        model_path: &Path,
        swap_dir: PathBuf,
        worker_slots: usize,
        max_hot_sessions: usize,
        max_hot_ram_bytes: usize,
        batch_limits: BatchLimits4,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
        hardware: HardwareConfig,
    ) -> anyhow::Result<Self>;
}

impl NetMuxServiceExt for NpuMuxService4 {
    fn open(
        model_path: &Path,
        swap_dir: PathBuf,
        worker_slots: usize,
        max_hot_sessions: usize,
        max_hot_ram_bytes: usize,
        batch_limits: BatchLimits4,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
    ) -> anyhow::Result<Self> {
        Self::open_with_hardware(
            model_path,
            swap_dir,
            worker_slots,
            max_hot_sessions,
            max_hot_ram_bytes,
            batch_limits,
            policy_mode,
            temperature,
            thinking_steps,
            HardwareConfig::default(),
        )
    }

    fn open_with_hardware(
        model_path: &Path,
        swap_dir: PathBuf,
        worker_slots: usize,
        max_hot_sessions: usize,
        max_hot_ram_bytes: usize,
        batch_limits: BatchLimits4,
        policy_mode: PolicyMode4,
        temperature: f32,
        thinking_steps: Option<u8>,
        hardware: HardwareConfig,
    ) -> anyhow::Result<Self> {
        let workers = NpuWorkerPool4::load_shared_with_hardware(
            model_path,
            worker_slots,
            batch_limits,
            policy_mode,
            temperature,
            thinking_steps,
            &hardware,
        )?;
        let mut config = MuxConfig4::new(swap_dir, workers.d_model());
        config.worker_slots = worker_slots;
        config.max_hot_sessions = max_hot_sessions;
        config.max_hot_ram_bytes = max_hot_ram_bytes;
        config.hardware = hardware;
        Self::with_workers(workers, config)
    }
}

#[cfg(test)]
pub(crate) fn worker_pool_without_graphs(
    worker_slots: usize,
    encoder_version: u32,
    d_model: usize,
) -> NpuWorkerPool4 {
    let workers = (0..worker_slots)
        .map(|_| NetPlayer4::without_graphs(encoder_version, d_model))
        .collect();
    NpuWorkerPool4::from_workers(workers).expect("worker_slots > 0")
}
