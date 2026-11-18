//! Search-advantage labels (`search_advantage_v1`, `convert4
//! --search-advantage`): at each searched priority root of a recorded seat,
//! the fork advantage of every answer the search tried over the policy's own,
//! with its standard error, beside the frames and never in them.
//!
//! A labelled root has a **twin**: its frame again, with the root's payable
//! menu (the policy's own menu and, after it, the spells the planner pays
//! for), in the dataset's split [`MENUS_SPLIT`] under the root's own
//! (game, seat, cursor). A twin's payload ends in exactly one
//! [`twin_block`]: [`TWIN_BLOCK_MAGIC`], the kind byte [`TWIN_KIND`], three
//! zero bytes, the root's cursor (`u32`, little-endian) and four zero bytes,
//! so a reader that does not know twins refuses one rather than taking it for
//! a decision of its own. The labels themselves are the sidecar
//! `search-advantage/` (`convert4`'s `search_advantage` module).

use anyhow::{Result, ensure};

/// The dataset contract of search-advantage labels.
pub const SEARCH_ADVANTAGE_CONTRACT: &str = "search_advantage_v1";

/// The split that holds the twins, beside `train` and `val`.
pub const MENUS_SPLIT: &str = "search-advantage-menus";

/// The `meta.json` key that lists the twins' shards.
pub const MENUS_SHARDS_KEY: &str = "search_advantage_menus_shards";

/// The split that holds a searched root's afterstates (`search_afterstate_v1`,
/// `convert4 --search-afterstates`): observation frames, one per searched
/// candidate whose afterstate is valid.
pub const AFTERSTATE_FRAMES_SPLIT: &str = "search-afterstate-frames";

/// The `meta.json` key that lists the afterstate frames' shards.
pub const AFTERSTATE_SHARDS_KEY: &str = "search_afterstates_shards";

/// The block a twin frame ends in, after its BST4 payload.
pub const TWIN_BLOCK_MAGIC: &[u8; 4] = b"BSA1";

/// The block's length.
pub const TWIN_BLOCK_LEN: usize = 16;

/// The kind byte of a twin: the root's payable menu.
pub const TWIN_KIND: u8 = 1;

/// A twin's block for the root frame at `root_cursor`.
#[must_use]
pub fn twin_block(root_cursor: u32) -> [u8; TWIN_BLOCK_LEN] {
    let mut block = [0; TWIN_BLOCK_LEN];
    block[..4].copy_from_slice(TWIN_BLOCK_MAGIC);
    block[4] = TWIN_KIND;
    block[8..12].copy_from_slice(&root_cursor.to_le_bytes());
    block
}

/// The root cursor a twin's block names, where `block` is one.
///
/// # Errors
/// A block of the right length and magic whose kind or reserved bytes are
/// not a twin's.
pub fn read_twin_block(block: &[u8]) -> Result<Option<u32>> {
    if block.len() != TWIN_BLOCK_LEN || !block.starts_with(TWIN_BLOCK_MAGIC) {
        return Ok(None);
    }
    ensure!(
        block[4] == TWIN_KIND && block[5..8] == [0, 0, 0] && block[12..16] == [0, 0, 0, 0],
        "a BSA1 block that is no twin's (kind {}, reserved bytes set)",
        block[4]
    );
    Ok(Some(u32::from_le_bytes(block[8..12].try_into()?)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_twin_block_reads_back_and_nothing_else_is_one() {
        let block = twin_block(0x0102_0304);
        assert_eq!(
            block,
            [b'B', b'S', b'A', b'1', 1, 0, 0, 0, 4, 3, 2, 1, 0, 0, 0, 0]
        );
        assert_eq!(read_twin_block(&block).unwrap(), Some(0x0102_0304));
        let mut kind = block;
        kind[4] = 2;
        assert!(read_twin_block(&kind).is_err());
        let mut reserved = block;
        reserved[13] = 1;
        assert!(read_twin_block(&reserved).is_err());
        assert_eq!(read_twin_block(&block[..15]).unwrap(), None);
        assert_eq!(
            read_twin_block(&crate::tactician_labels::distill_block(1, 2)).unwrap(),
            None
        );
    }
}
