//! Native trajectory ABI, bounded shard readers, cache construction and admission.
pub mod cache;
pub mod contracts;
pub mod packing;
mod reader;
pub use reader::read_shard;
#[cfg(test)]
mod tests;
