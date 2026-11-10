//! The one ONNX Runtime environment the process's sessions are built in.
//!
//! Delegated to [`baylee_driver::ort_env::environment`] so training harnesses
//! and client/NPU runtime drivers share a single process-wide ORT environment.

use ort::environment::Environment;

/// The process's environment, built on first use.
///
/// # Errors
///
/// When ONNX Runtime cannot make one; the next call tries again.
pub fn environment() -> Result<&'static Environment, ort::Error> {
    baylee_driver::ort_env::environment()
}
