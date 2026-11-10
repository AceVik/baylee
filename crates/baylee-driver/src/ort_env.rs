//! Process-wide ONNX Runtime [`Environment`] and multi-hardware [`Session`] factory.

use std::path::Path;
use std::sync::OnceLock;

use anyhow::Context as _;
use ort::environment::Environment;
use ort::ep::ExecutionProviderDispatch;
use ort::session::Session;
use ort::session::builder::GraphOptimizationLevel;

use crate::hardware::{AcceleratorKind, HardwareConfig};

fn ort_error(err: impl std::fmt::Display) -> anyhow::Error {
    anyhow::anyhow!("{err}")
}

/// Returns the process-wide ONNX Runtime [`Environment`], initializing it on first use.
///
/// # Errors
/// When ONNX Runtime fails to initialize its global environment.
pub fn environment() -> Result<&'static Environment, ort::Error> {
    static ENVIRONMENT: OnceLock<Environment> = OnceLock::new();
    if let Some(env) = ENVIRONMENT.get() {
        return Ok(env);
    }
    let made = ort::init().with_name("baylee-driver").build()?;
    Ok(ENVIRONMENT.get_or_init(|| made))
}

#[allow(
    dead_code,
    reason = "Used when one or more hardware EP features are enabled"
)]
fn dispatch_policy(ep: ExecutionProviderDispatch, strict: bool) -> ExecutionProviderDispatch {
    if strict {
        ep.error_on_failure()
    } else {
        ep.fail_silently()
    }
}

#[allow(
    unused_variables,
    unused_mut,
    clippy::too_many_lines,
    reason = "Hardware dispatch arms depend on which accelerator features are compiled"
)]
fn build_execution_providers(
    hw: &HardwareConfig,
    accel: AcceleratorKind,
) -> anyhow::Result<Vec<ExecutionProviderDispatch>> {
    let strict = !hw.graceful_cpu_fallback;
    let mut eps = Vec::new();
    match accel {
        AcceleratorKind::OpenVino => {
            #[cfg(feature = "openvino")]
            {
                eps.push(dispatch_policy(
                    ort::ep::OpenVINO::default()
                        .with_device_type(&hw.openvino_device)
                        .build(),
                    strict,
                ));
            }
            #[cfg(not(feature = "openvino"))]
            anyhow::ensure!(
                !strict,
                "Accelerator 'openvino' requested, but baylee-driver was built without --features openvino"
            );
        }
        AcceleratorKind::CoreMl => {
            #[cfg(feature = "coreml")]
            {
                eps.push(dispatch_policy(ort::ep::CoreML::default().build(), strict));
            }
            #[cfg(not(feature = "coreml"))]
            anyhow::ensure!(
                !strict,
                "Accelerator 'coreml' requested, but baylee-driver was built without --features coreml"
            );
        }
        AcceleratorKind::DirectMl => {
            #[cfg(feature = "directml")]
            {
                eps.push(dispatch_policy(
                    ort::ep::DirectML::default().build(),
                    strict,
                ));
            }
            #[cfg(not(feature = "directml"))]
            anyhow::ensure!(
                !strict,
                "Accelerator 'directml' requested, but baylee-driver was built without --features directml"
            );
        }
        AcceleratorKind::TensorRt => {
            #[cfg(feature = "tensorrt")]
            {
                eps.push(dispatch_policy(
                    ort::ep::TensorRT::default().build(),
                    strict,
                ));
            }
            #[cfg(not(feature = "tensorrt"))]
            anyhow::ensure!(
                !strict,
                "Accelerator 'tensorrt' requested, but baylee-driver was built without --features tensorrt"
            );
        }
        AcceleratorKind::Cuda => {
            #[cfg(feature = "cuda")]
            {
                eps.push(dispatch_policy(
                    ort::ep::CUDA::default().with_tf32(hw.allow_tf32).build(),
                    strict,
                ));
            }
            #[cfg(not(feature = "cuda"))]
            anyhow::ensure!(
                !strict,
                "BAYLEE_EP=cuda requires a binary built with --features cuda / onnx-cuda"
            );
        }
        AcceleratorKind::Rocm => {
            #[cfg(feature = "rocm")]
            {
                eps.push(dispatch_policy(ort::ep::ROCm::default().build(), strict));
            }
            #[cfg(not(feature = "rocm"))]
            anyhow::ensure!(
                !strict,
                "Accelerator 'rocm' requested, but baylee-driver was built without --features rocm"
            );
        }
        AcceleratorKind::MiGraphX => {
            #[cfg(feature = "migraphx")]
            {
                eps.push(dispatch_policy(
                    ort::ep::MIGraphX::default().build(),
                    strict,
                ));
            }
            #[cfg(not(feature = "migraphx"))]
            anyhow::ensure!(
                !strict,
                "Accelerator 'migraphx' requested, but baylee-driver was built without --features migraphx"
            );
        }
        AcceleratorKind::Qnn => {
            #[cfg(feature = "qnn")]
            {
                eps.push(dispatch_policy(ort::ep::QNN::default().build(), strict));
            }
            #[cfg(not(feature = "qnn"))]
            anyhow::ensure!(
                !strict,
                "Accelerator 'qnn' requested, but baylee-driver was built without --features qnn"
            );
        }
        AcceleratorKind::NnApi => {
            #[cfg(feature = "nnapi")]
            {
                eps.push(dispatch_policy(ort::ep::NNAPI::default().build(), strict));
            }
            #[cfg(not(feature = "nnapi"))]
            anyhow::ensure!(
                !strict,
                "Accelerator 'nnapi' requested, but baylee-driver was built without --features nnapi"
            );
        }
        AcceleratorKind::WebGpu => {
            #[cfg(feature = "webgpu")]
            {
                eps.push(dispatch_policy(ort::ep::WebGPU::default().build(), strict));
            }
            #[cfg(not(feature = "webgpu"))]
            anyhow::ensure!(
                !strict,
                "Accelerator 'webgpu' requested, but baylee-driver was built without --features webgpu"
            );
        }
        AcceleratorKind::OneDnn => {
            #[cfg(feature = "onednn")]
            {
                eps.push(dispatch_policy(ort::ep::OneDNN::default().build(), strict));
            }
            #[cfg(not(feature = "onednn"))]
            anyhow::ensure!(
                !strict,
                "Accelerator 'onednn' requested, but baylee-driver was built without --features onednn"
            );
        }
        AcceleratorKind::Xnnpack => {
            #[cfg(feature = "xnnpack")]
            {
                eps.push(dispatch_policy(ort::ep::XNNPACK::default().build(), strict));
            }
            #[cfg(not(feature = "xnnpack"))]
            anyhow::ensure!(
                !strict,
                "Accelerator 'xnnpack' requested, but baylee-driver was built without --features xnnpack"
            );
        }
        AcceleratorKind::Auto | AcceleratorKind::Cpu => {}
    }
    Ok(eps)
}

/// Builds and commits an ONNX Runtime [`Session`] from `path` configured for `hw`.
///
/// # Errors
/// When the session builder, execution provider registration, or model loading fails.
pub fn build_session(
    path: &Path,
    hw: &HardwareConfig,
    worker_slots: usize,
) -> anyhow::Result<Session> {
    let accel = hw.effective_accelerator();
    let eps = build_execution_providers(hw, accel)?;
    let intra_threads = hw.recommended_intra_threads(worker_slots);
    let mut builder = Session::builder(environment().map_err(ort_error)?).map_err(ort_error)?;
    if !eps.is_empty() {
        builder = builder.with_execution_providers(eps).map_err(ort_error)?;
    }
    builder
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(ort_error)?
        .with_intra_threads(intra_threads)
        .map_err(ort_error)?
        .commit_from_file(path)
        .with_context(|| format!("loading {}", path.display()))
}
