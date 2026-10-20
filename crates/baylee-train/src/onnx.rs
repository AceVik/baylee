//! The one ONNX Runtime environment the process's sessions are built in.
//!
//! Since `ort` 2.0.0-rc.15 a session is built against an explicit
//! [`Environment`]; earlier versions made one on first use. One per process
//! keeps that behaviour: every net, batcher and league seat shares its
//! thread pools and logger.

use std::sync::OnceLock;

use ort::environment::Environment;

/// The process's environment, built on first use.
///
/// # Errors
///
/// When ONNX Runtime cannot make one; the next call tries again.
pub fn environment() -> Result<&'static Environment, ort::Error> {
    static ENVIRONMENT: OnceLock<Environment> = OnceLock::new();
    if let Some(environment) = ENVIRONMENT.get() {
        return Ok(environment);
    }
    let made = ort::init().with_name("baylee-train").build()?;
    Ok(ENVIRONMENT.get_or_init(|| made))
}
