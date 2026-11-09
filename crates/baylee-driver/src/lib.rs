//! `baylee-driver` — Multi-hardware Baylee AI runtime driver & tiered NPU/GPU/CPU session multiplexer.
//!
//! Designed to be compiled both into the dedicated NPU server (`acenb`) and directly
//! into player client binaries across Windows (`DirectML`, `CUDA`, `OpenVINO`),
//! macOS/iOS (`CoreML` Apple Neural Engine / M-series GPU), Linux (`OpenVINO`,
//! `ROCm`, `CUDA`, `OneDNN`), Android (`QNN`, `NNAPI`), and Web (`WebGPU`),
//! without pulling in any training-only dependencies (`baylee-train`).

#![warn(missing_docs)]

pub mod config;
pub mod fp16;
pub mod hardware;
pub mod metrics;
pub mod mux;
#[cfg(feature = "onnx")]
pub mod ort_env;
pub mod scheduler;
pub mod snapshot;
pub mod swap_store;
pub mod worker_pool;

pub use config::{MuxConfig4, SessionUrgency4};
pub use hardware::{AcceleratorKind, GraphPrecision, HardwareConfig};
pub use metrics::{MuxMetrics4, MuxSummary4};
pub use mux::{ActWaveItem4, NpuMuxService4, ObserveWaveItem4};
pub use scheduler::TieredSessionPool4;
pub use snapshot::{SnapshottableBrain, SnapshottableState};
pub use swap_store::{EncryptedSwapStore4, SwapHandle4};
pub use worker_pool::{NpuWorkerPool4, WaveSlotOutput4};
