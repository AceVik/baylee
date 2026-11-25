//! Native trajectory ABI, bounded shard readers, cache construction and admission.
use crate::search_advantage::{
    AFTERSTATE_FRAMES_SPLIT, AFTERSTATE_SHARDS_KEY, MENUS_SHARDS_KEY, MENUS_SPLIT,
};

pub mod cache;
pub mod contracts;
pub mod packing;
mod reader;
pub use reader::{read_shard, read_twins};
#[cfg(test)]
mod tests;

/// A split of a dataset.
pub struct Split {
    /// The directory its shards lie in.
    pub name: &'static str,
    /// The `meta.json` key that lists its shards.
    pub shards_key: &'static str,
    /// Whether its frames are search-advantage twins, which end in a BSA1
    /// block that no frame of another split may end in.
    pub twins: bool,
}

/// Every split a dataset may have: a search-advantage dataset's twins and its
/// afterstates are splits of their own.
pub static SPLITS: [Split; 4] = [
    Split {
        name: "train",
        shards_key: "train_shards",
        twins: false,
    },
    Split {
        name: "val",
        shards_key: "val_shards",
        twins: false,
    },
    Split {
        name: MENUS_SPLIT,
        shards_key: MENUS_SHARDS_KEY,
        twins: true,
    },
    Split {
        name: AFTERSTATE_FRAMES_SPLIT,
        shards_key: AFTERSTATE_SHARDS_KEY,
        twins: false,
    },
];
