//! Immutable complete-game packages; every training recipe is an explicit manifest.
#[cfg(target_os = "linux")]
mod consumer;
mod mixing;
mod producer;
mod publish;
mod recover;
#[cfg(test)]
mod tests;

#[cfg(target_os = "linux")]
pub use consumer::{Consumer, consume};
pub use mixing::select_entries;
pub use producer::{Producer, produce};
pub use publish::{publish, verify_index};
pub use recover::{Recovery, recover};
