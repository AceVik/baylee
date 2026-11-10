//! Multi-hardware Execution Provider discovery, precision routing, and thread
//! topology for client devices (macOS, Windows, Linux, mobile) and NPU servers.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::bail;
use serde::{Deserialize, Serialize};

/// Target hardware execution backend for Baylee AI inference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceleratorKind {
    /// Automatically probes and selects the highest-priority accelerator
    /// compiled and available on the host (NPU -> GPU -> SIMD CPU).
    #[default]
    Auto,
    /// Intel AI Boost NPU / Arc GPU via `OpenVINO` (`acenb` Meteor Lake / Lunar Lake).
    OpenVino,
    /// Apple Neural Engine (ANE) and Apple Silicon GPU via `CoreML` (macOS / iOS).
    CoreMl,
    /// Windows DirectX 12 GPU and NPU via `DirectML` (AMD, Intel Arc, NVIDIA, Snapdragon X).
    DirectMl,
    /// NVIDIA `TensorRT` engine execution provider.
    TensorRt,
    /// NVIDIA CUDA execution provider (`bigace` RTX 4070 Ti and player `GeForce` GPUs).
    Cuda,
    /// AMD `ROCm` execution provider.
    Rocm,
    /// AMD `MIGraphX` execution provider.
    MiGraphX,
    /// Qualcomm AI Engine Direct (Hexagon NPU).
    Qnn,
    /// Android Neural Networks API (mobile NPU/GPU).
    NnApi,
    /// Cross-platform WebGPU compute backend.
    WebGpu,
    /// Intel oneDNN SIMD accelerator.
    OneDnn,
    /// ARM/x86 XNNPACK SIMD accelerator.
    Xnnpack,
    /// Portable x86-64 AVX2 / ARM64 NEON CPU execution provider.
    Cpu,
}

impl AcceleratorKind {
    /// Short identifier used in CLI flags, `BAYLEE_EP`, and telemetry reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::OpenVino => "openvino",
            Self::CoreMl => "coreml",
            Self::DirectMl => "directml",
            Self::TensorRt => "tensorrt",
            Self::Cuda => "cuda",
            Self::Rocm => "rocm",
            Self::MiGraphX => "migraphx",
            Self::Qnn => "qnn",
            Self::NnApi => "nnapi",
            Self::WebGpu => "webgpu",
            Self::OneDnn => "onednn",
            Self::Xnnpack => "xnnpack",
            Self::Cpu => "cpu",
        }
    }

    /// Whether this execution provider executes IEEE-754 `FP16` (`MLFloat16`)
    /// graphs natively in hardware without per-operator software casts.
    ///
    /// Note: x86-64 `CPUExecutionProvider` (AVX2) executes `FP32` FMA natively
    /// and emulates `MLFloat16` via software casts, so `Cpu` and `OneDnn`
    /// prefer the `FP32` ONNX graph while still storing session `h_states` in
    /// `FP16` in RAM and on SSD.
    #[must_use]
    pub const fn prefers_fp16_graph(self) -> bool {
        match self {
            Self::OpenVino
            | Self::CoreMl
            | Self::DirectMl
            | Self::TensorRt
            | Self::Cuda
            | Self::Rocm
            | Self::MiGraphX
            | Self::Qnn
            | Self::NnApi
            | Self::WebGpu => true,
            Self::Auto | Self::OneDnn | Self::Xnnpack | Self::Cpu => false,
        }
    }

    /// Resolves [`AcceleratorKind::Auto`] to the best compiled backend for the
    /// current operating system and feature set.
    #[must_use]
    pub fn resolve_auto(self) -> Self {
        if self != Self::Auto {
            return self;
        }
        #[cfg(feature = "openvino")]
        {
            return Self::OpenVino;
        }
        #[cfg(all(not(feature = "openvino"), feature = "coreml", target_os = "macos"))]
        {
            return Self::CoreMl;
        }
        #[cfg(all(
            not(feature = "openvino"),
            not(all(feature = "coreml", target_os = "macos")),
            feature = "directml",
            target_os = "windows"
        ))]
        {
            return Self::DirectMl;
        }
        #[cfg(all(
            not(feature = "openvino"),
            not(all(feature = "coreml", target_os = "macos")),
            not(all(feature = "directml", target_os = "windows")),
            feature = "tensorrt"
        ))]
        {
            return Self::TensorRt;
        }
        #[cfg(all(
            not(feature = "openvino"),
            not(all(feature = "coreml", target_os = "macos")),
            not(all(feature = "directml", target_os = "windows")),
            not(feature = "tensorrt"),
            feature = "cuda"
        ))]
        {
            return Self::Cuda;
        }
        #[cfg(all(
            not(feature = "openvino"),
            not(all(feature = "coreml", target_os = "macos")),
            not(all(feature = "directml", target_os = "windows")),
            not(feature = "tensorrt"),
            not(feature = "cuda"),
            feature = "rocm"
        ))]
        {
            return Self::Rocm;
        }
        #[cfg(all(
            not(feature = "openvino"),
            not(all(feature = "coreml", target_os = "macos")),
            not(all(feature = "directml", target_os = "windows")),
            not(feature = "tensorrt"),
            not(feature = "cuda"),
            not(feature = "rocm"),
            feature = "webgpu"
        ))]
        {
            return Self::WebGpu;
        }
        #[allow(unreachable_code)]
        Self::Cpu
    }
}

impl FromStr for AcceleratorKind {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" | "" => Ok(Self::Auto),
            "npu" | "openvino" | "intel_npu" | "intel-npu" => Ok(Self::OpenVino),
            "coreml" | "ane" | "metal" | "apple" => Ok(Self::CoreMl),
            "directml" | "dml" | "dx12" => Ok(Self::DirectMl),
            "tensorrt" | "trt" => Ok(Self::TensorRt),
            "cuda" | "nvidia" => Ok(Self::Cuda),
            "rocm" | "hip" => Ok(Self::Rocm),
            "migraphx" => Ok(Self::MiGraphX),
            "qnn" | "qualcomm" | "snapdragon" => Ok(Self::Qnn),
            "nnapi" | "android" => Ok(Self::NnApi),
            "webgpu" | "wgpu" => Ok(Self::WebGpu),
            "onednn" | "dnnl" => Ok(Self::OneDnn),
            "xnnpack" => Ok(Self::Xnnpack),
            "cpu" | "avx2" | "neon" => Ok(Self::Cpu),
            other => bail!("unknown hardware accelerator '{other}'"),
        }
    }
}

/// Model graph precision selection policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphPrecision {
    /// Automatically picks `fp16` on NPU/GPU/ANE hardware and `fp32` on x86 AVX2 CPU.
    #[default]
    Auto,
    /// Forces the `FP16` (`175 MB`) ONNX graph if available.
    Fp16,
    /// Forces the `FP32` (`350 MB`) ONNX graph.
    Fp32,
}

/// Hardware execution configuration for a Baylee AI driver or multiplexer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareConfig {
    /// Target hardware accelerator (`Auto` probes NPU -> GPU -> CPU).
    pub accelerator: AcceleratorKind,
    /// Graph precision policy (`Auto`, `Fp16`, or `Fp32`).
    pub precision: GraphPrecision,
    /// `OpenVINO` device string when `OpenVino` is active (e.g. `"NPU"`, `"GPU"`, `"AUTO:NPU,GPU,CPU"`).
    pub openvino_device: String,
    /// Intra-op threads per inference session (`0` = automatic topology sizing).
    pub intra_threads: usize,
    /// Whether CUDA TF32 tensor cores are allowed (`false` for strict rollout parity,
    /// `true` for live client/server play).
    pub allow_tf32: bool,
    /// Whether to fall back gracefully to `CPUExecutionProvider` if the
    /// requested hardware accelerator fails to initialize on a player's device.
    pub graceful_cpu_fallback: bool,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        let accelerator = std::env::var("BAYLEE_EP")
            .ok()
            .and_then(|val| AcceleratorKind::from_str(&val).ok())
            .unwrap_or(AcceleratorKind::Auto);
        Self {
            accelerator,
            precision: GraphPrecision::Auto,
            openvino_device: "AUTO:NPU,GPU,CPU".to_owned(),
            intra_threads: 0,
            allow_tf32: false,
            graceful_cpu_fallback: true,
        }
    }
}

impl HardwareConfig {
    /// Effective accelerator after resolving `Auto`.
    #[must_use]
    pub fn effective_accelerator(&self) -> AcceleratorKind {
        self.accelerator.resolve_auto()
    }

    /// Computes the optimal intra-op thread count per worker session given
    /// `worker_slots` concurrent sessions.
    #[must_use]
    pub fn recommended_intra_threads(&self, worker_slots: usize) -> usize {
        if self.intra_threads > 0 {
            return self.intra_threads;
        }
        let accel = self.effective_accelerator();
        if accel != AcceleratorKind::Cpu && accel != AcceleratorKind::OneDnn {
            return 1;
        }
        let cores = std::thread::available_parallelism().map_or(4, std::num::NonZeroUsize::get);
        let slots = worker_slots.max(1);
        (cores / slots).clamp(1, 4)
    }

    /// Resolves the best ONNX model directory (`model/` vs `model/fp16` or
    /// `model-fp16/`) for the active hardware accelerator.
    #[must_use]
    pub fn resolve_model_dir(&self, requested: &Path) -> PathBuf {
        let want_fp16 = match self.precision {
            GraphPrecision::Fp16 => true,
            GraphPrecision::Fp32 => false,
            GraphPrecision::Auto => self.effective_accelerator().prefers_fp16_graph(),
        };
        if !want_fp16 {
            return requested.to_path_buf();
        }
        let nested_fp16 = requested.join("fp16");
        if nested_fp16.join("model.onnx.json").is_file() {
            return nested_fp16;
        }
        if let Some(name) = requested.file_name().and_then(|n| n.to_str()) {
            let sibling = requested.with_file_name(format!("{name}-fp16"));
            if sibling.join("model.onnx.json").is_file() {
                return sibling;
            }
        }
        requested.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_accelerator_aliases_and_precision_preferences() {
        assert_eq!(
            AcceleratorKind::from_str("npu").unwrap(),
            AcceleratorKind::OpenVino
        );
        assert_eq!(
            AcceleratorKind::from_str("coreml").unwrap(),
            AcceleratorKind::CoreMl
        );
        assert_eq!(
            AcceleratorKind::from_str("directml").unwrap(),
            AcceleratorKind::DirectMl
        );
        assert_eq!(
            AcceleratorKind::from_str("cuda").unwrap(),
            AcceleratorKind::Cuda
        );
        assert_eq!(
            AcceleratorKind::from_str("cpu").unwrap(),
            AcceleratorKind::Cpu
        );
        assert!(AcceleratorKind::OpenVino.prefers_fp16_graph());
        assert!(AcceleratorKind::CoreMl.prefers_fp16_graph());
        assert!(AcceleratorKind::DirectMl.prefers_fp16_graph());
        assert!(AcceleratorKind::Cuda.prefers_fp16_graph());
        assert!(!AcceleratorKind::Cpu.prefers_fp16_graph());
    }
}
