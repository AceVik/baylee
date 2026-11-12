//! The tactical layer's answers kept as labels (`docs/trained-ai.md` §"The
//! tactician, stage S1") are read only where they were made for:
//!
//! - [`IMITATION_CONTRACT`] (`eval4 --tactician-imitate`, `--tactician-relabel`):
//!   the tactician's records, read only by its own trainer;
//! - [`DISTILL_CONTRACT`] (`eval4 --distill-labels`): the whole net's
//!   distillation labels beside the game records, read only by `convert4
//!   --bundle-distill` ([`admit_distillation`]);
//! - [`HOUSE_DISTILL_CONTRACT`] (`eval4 --distill-teacher house:<level>`):
//!   the same labels with a fair house level's answers in place of the
//!   layer's, read only by the same reader.
//!
//! A run that keeps either names its contract in its `run.json`. Every reader
//! of a run that turns it into training data of another kind (convert4, the
//! research and self-imitation readers, the corpus indexes) calls
//! [`refuse_imitation`] on it first.
//!
//! A distillation dataset's frames may end in a [`distill_block`] after
//! their BST4 payload; [`trailing_distill_block`] is how a reader that checks
//! a frame's length admits it and nothing else.

use anyhow::{Result, ensure};
use serde_json::Value;

/// The record and run contract of the tactician's imitation labels.
pub const IMITATION_CONTRACT: &str = "tactician_imitation_v1";

/// The run and dataset contract of the whole net's distillation labels.
pub const DISTILL_CONTRACT: &str = "bundle_distillation_v1";

/// The run and dataset contract of the house-teacher labels: the distillation
/// labels with a fair house level's answer `H` where the layer's `R` was.
pub const HOUSE_DISTILL_CONTRACT: &str = "house_distillation_v1";

/// The contracts `convert4 --bundle-distill` admits, one per dataset.
pub const DISTILL_CONTRACTS: [&str; 2] = [DISTILL_CONTRACT, HOUSE_DISTILL_CONTRACT];

/// The block a distillation dataset appends to a recorded seat's decision
/// frame, after the BST4 payload (the free mask): this magic, the priority
/// subtype's code (`u8`, [`NO_SUBTYPE`] for none), three zero bytes, and the
/// mask of the rule groups that fired (`u64`, little-endian; the dataset's
/// `meta.json` names the codes and the bits).
pub const DISTILL_BLOCK_MAGIC: &[u8; 4] = b"BDX1";

/// The block's length.
pub const DISTILL_BLOCK_LEN: usize = 16;

/// The subtype code of a frame with none (not a priority label, or no label).
pub const NO_SUBTYPE: u8 = 255;

/// A distillation block's bytes.
#[must_use]
pub fn distill_block(subtype: u8, fired: u64) -> [u8; DISTILL_BLOCK_LEN] {
    let mut block = [0; DISTILL_BLOCK_LEN];
    block[..4].copy_from_slice(DISTILL_BLOCK_MAGIC);
    block[4] = subtype;
    block[8..].copy_from_slice(&fired.to_le_bytes());
    block
}

/// What follows a BST4 payload: nothing, or exactly one distillation block,
/// read as `(subtype, fired)`.
///
/// # Errors
/// Any other trailing bytes: a wrong length, magic or reserved byte.
pub fn trailing_distill_block(trailing: &[u8]) -> Result<Option<(u8, u64)>> {
    if trailing.is_empty() {
        return Ok(None);
    }
    ensure!(
        trailing.len() == DISTILL_BLOCK_LEN
            && trailing.starts_with(DISTILL_BLOCK_MAGIC)
            && trailing[5..8] == [0, 0, 0],
        "BST4 frame has {} trailing bytes that are no distillation block",
        trailing.len()
    );
    Ok(Some((
        trailing[4],
        u64::from_le_bytes(trailing[8..].try_into()?),
    )))
}

/// Refuses a run whose `run.json` keeps the layer's answers as labels.
///
/// # Errors
/// When any value in `run`, at any depth, is [`IMITATION_CONTRACT`] or one
/// of [`DISTILL_CONTRACTS`].
pub fn refuse_imitation(run: &Value) -> Result<()> {
    ensure!(
        !names(run, IMITATION_CONTRACT),
        "this run keeps {IMITATION_CONTRACT} records: the tactical layer's answers as labels, \
         which train the tactician and nothing else"
    );
    for contract in DISTILL_CONTRACTS {
        ensure!(
            !names(run, contract),
            "this run keeps {contract} labels: a teacher's answers, read only by convert4 \
             --bundle-distill"
        );
    }
    Ok(())
}

/// Admits a run to `convert4 --bundle-distill`: its `run.json` must declare
/// one of the distillation contracts (`distill.contract`), which is
/// returned. The tactician's imitation contract it may also name is for its
/// own records, elsewhere.
///
/// # Errors
/// A run that declares none of [`DISTILL_CONTRACTS`].
pub fn admit_distillation(run: &Value) -> Result<&'static str> {
    DISTILL_CONTRACTS
        .into_iter()
        .find(|contract| run["distill"]["contract"] == *contract)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "convert4 --bundle-distill reads only runs recorded with eval4 --distill-labels \
                 ({DISTILL_CONTRACT}) or --distill-teacher ({HOUSE_DISTILL_CONTRACT})"
            )
        })
}

/// Whether any string in `value`, at any depth, is `contract`.
fn names(value: &Value, contract: &str) -> bool {
    match value {
        Value::String(text) => text == contract,
        Value::Array(items) => items.iter().any(|item| names(item, contract)),
        Value::Object(map) => map.values().any(|item| names(item, contract)),
        Value::Null | Value::Bool(_) | Value::Number(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_run_naming_either_contract_anywhere_is_refused() {
        refuse_imitation(&json!({"tactician": {"record_contract": "tactician_records_v1"}}))
            .unwrap();
        for run in [
            json!({"labels": "tactician_imitation_v1"}),
            json!({"tactician": {"record_contract": "tactician_imitation_v1"}}),
            json!({"runs": [{"contracts": ["x", "tactician_imitation_v1"]}]}),
            json!({"distill": {"contract": "bundle_distillation_v1"}}),
            json!({"distill": {"contract": "house_distillation_v1"}}),
        ] {
            assert!(refuse_imitation(&run).is_err());
        }
    }

    #[test]
    fn a_distillation_block_reads_back_and_nothing_else_trails_a_frame() {
        let block = distill_block(3, 1 << 8 | 1);
        assert_eq!(&block[..4], b"BDX1");
        assert_eq!(trailing_distill_block(&block).unwrap(), Some((3, 257)));
        assert_eq!(
            trailing_distill_block(&distill_block(NO_SUBTYPE, 0)).unwrap(),
            Some((255, 0))
        );
        assert_eq!(trailing_distill_block(&[]).unwrap(), None);
        let mut reserved = block;
        reserved[6] = 1;
        let mut magic = block;
        magic[3] = b'2';
        for wrong in [&block[..15], &reserved[..], &magic[..], &[0; 16][..]] {
            assert!(trailing_distill_block(wrong).is_err());
        }
        let mut longer = block.to_vec();
        longer.push(0);
        assert!(trailing_distill_block(&longer).is_err());
    }

    #[test]
    fn only_a_declared_distillation_run_is_admitted_to_its_mode() {
        let layer = admit_distillation(&json!({"distill": {"contract": "bundle_distillation_v1"},
            "labels": "tactician_imitation_v1"}))
        .unwrap();
        assert_eq!(layer, DISTILL_CONTRACT);
        let house =
            admit_distillation(&json!({"distill": {"contract": "house_distillation_v1"}})).unwrap();
        assert_eq!(house, HOUSE_DISTILL_CONTRACT);
        for run in [
            json!({"labels": "tactician_imitation_v1"}),
            json!({"distill": {"contract": "search_distillation_v1"}}),
            json!({}),
        ] {
            assert!(admit_distillation(&run).is_err());
        }
    }
}
