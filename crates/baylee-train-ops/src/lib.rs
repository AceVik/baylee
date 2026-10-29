//! Native training administration. Rules and neural tensor math live elsewhere.
pub mod files;
pub mod trajectory;
pub mod evaluation;
#[cfg(target_os = "linux")]
pub mod guard;
#[cfg(target_os = "linux")]
pub mod process;
