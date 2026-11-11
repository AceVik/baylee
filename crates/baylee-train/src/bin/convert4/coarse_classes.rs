//! The coarse classes of a distillation dataset's labelled priority frames
//! (`--coarse-classes DIR`, rule `coarse_class_v1`): beside the frames, never
//! in them, so the shards and every cache keyed on them stay as they are.
//!
//! Four files, keyed by (game, seat, cursor), the store index's own key:
//!
//! - `coarse-classes.index.bin`: one 32-byte little-endian row per frame,
//!   sorted by game, seat and cursor ([`ROW_BYTES`], `dtype` in the header):
//!   game `u32`, cursor `u32`, seat `u8`, `R`'s class `u8`, `N`'s class `u8`,
//!   the label's subtype code `u8` (255 none), `R`'s candidate `u32` (the
//!   frame's target), `N`'s candidate `u32`, the candidate count `u32`, and
//!   the offset of its classes in the blob `u64`;
//! - `coarse-classes.played.bin` (version 2): one 8-byte row per index row,
//!   in the same order ([`PLAYED_ROW_BYTES`]): the played answer's candidate
//!   `u32` (the record's, `u32::MAX` none), its class `u8`, `free` `u8` (1
//!   where nothing was owed: a free priority, as `eval4`'s teacher counts
//!   define it) and two zero bytes. A file of its own, so a reader of
//!   version 1 reads the index as before;
//! - `coarse-classes.classes.bin`: every frame's candidate classes, one byte
//!   each in menu order, concatenated;
//! - `coarse-classes.json`: the header: the rule, the class names, the
//!   dtype, the row count, the files' SHA-256 and the counts.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context as _, ensure};
use baylee_train::coarse_class::{COARSE_CLASS_RULE, COARSE_CLASSES, NO_CLASS};
use baylee_train::convert4::Step4;
use baylee_train::convert4::distill_labels::DistillSubtype4;
use serde_json::{Value, json};

/// The bytes of one index row.
pub(super) const ROW_BYTES: usize = 32;

/// The bytes of one row of the played answers.
pub(super) const PLAYED_ROW_BYTES: usize = 8;

/// The layout's version: 2 added the played answers' file.
pub(super) const VERSION: u32 = 2;

/// One labelled priority frame's classes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Row {
    game: u32,
    cursor: u32,
    seat: u8,
    subtype: u8,
    r_idx: u32,
    n_idx: u32,
    p_idx: u32,
    free: bool,
    n_ne_r: bool,
    classes: Vec<u8>,
}

impl Row {
    /// The row of a frame, when it is a labelled priority frame with classes.
    pub(super) fn of(step: &Step4) -> Option<Self> {
        let label = step.distill.as_ref()?.label.as_ref()?;
        if label.label_kind != "priority" {
            return None;
        }
        Some(Self {
            game: step.game_id,
            cursor: step.step_cursor,
            seat: step.seat,
            subtype: label.subtype.map_or(NO_CLASS, DistillSubtype4::code),
            r_idx: u32::from(step.target.single_idx),
            n_idx: label.net_answer.single_idx.map_or(u32::MAX, u32::from),
            p_idx: label.played.map_or(u32::MAX, u32::from),
            free: label.subtype != Some(DistillSubtype4::Pay),
            n_ne_r: label.n_ne_r,
            classes: step.cand_classes.clone()?,
        })
    }

    fn class(&self, index: u32) -> u8 {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.classes.get(i).copied())
            .unwrap_or(NO_CLASS)
    }
}

fn class_name(code: u8) -> &'static str {
    COARSE_CLASSES
        .get(usize::from(code))
        .copied()
        .unwrap_or("none")
}

/// Writes the three files under `dir` and returns the header.
///
/// # Errors
/// When a file exists already or cannot be written.
pub(super) fn write(dir: &Path, mut rows: Vec<Row>, extra: &Value) -> anyhow::Result<Value> {
    fs::create_dir_all(dir)?;
    let index_path = dir.join("coarse-classes.index.bin");
    let blob_path = dir.join("coarse-classes.classes.bin");
    let header_path = dir.join("coarse-classes.json");
    let played_path = dir.join("coarse-classes.played.bin");
    for path in [&index_path, &blob_path, &header_path, &played_path] {
        ensure!(!path.exists(), "{} exists", path.display());
    }
    rows.sort_by_key(|row| (row.game, row.seat, row.cursor));
    let mut index = Vec::with_capacity(rows.len() * ROW_BYTES);
    let mut blob = Vec::new();
    let mut played = Vec::with_capacity(rows.len() * PLAYED_ROW_BYTES);
    let mut n_ne_r: BTreeMap<String, u64> = BTreeMap::new();
    let mut n_eq_r: BTreeMap<String, u64> = BTreeMap::new();
    let mut by_subtype: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();
    // At the free priorities: R's, N's and the played answer's classes.
    let mut at_free: BTreeMap<&str, BTreeMap<String, u64>> = BTreeMap::new();
    let mut free_rows = 0_u64;
    for row in &rows {
        let (r_class, n_class) = (row.class(row.r_idx), row.class(row.n_idx));
        let p_class = row.class(row.p_idx);
        index.extend_from_slice(&row.game.to_le_bytes());
        index.extend_from_slice(&row.cursor.to_le_bytes());
        index.extend_from_slice(&[row.seat, r_class, n_class, row.subtype]);
        index.extend_from_slice(&row.r_idx.to_le_bytes());
        index.extend_from_slice(&row.n_idx.to_le_bytes());
        index.extend_from_slice(&u32::try_from(row.classes.len())?.to_le_bytes());
        index.extend_from_slice(&(blob.len() as u64).to_le_bytes());
        played.extend_from_slice(&row.p_idx.to_le_bytes());
        played.extend_from_slice(&[p_class, u8::from(row.free), 0, 0]);
        blob.extend_from_slice(&row.classes);
        if row.free {
            free_rows += 1;
            for (who, class) in [("r", r_class), ("n", n_class), ("p", p_class)] {
                *at_free
                    .entry(who)
                    .or_default()
                    .entry(class_name(class).to_owned())
                    .or_default() += 1;
            }
        }
        let pair = format!("R {} / N {}", class_name(r_class), class_name(n_class));
        let counts = if row.n_ne_r { &mut n_ne_r } else { &mut n_eq_r };
        *counts.entry(pair).or_default() += 1;
        let subtype = DistillSubtype4::ALL
            .iter()
            .find(|s| s.code() == row.subtype)
            .map_or("none", |s| s.name());
        *by_subtype
            .entry(subtype.to_owned())
            .or_default()
            .entry(class_name(r_class).to_owned())
            .or_default() += 1;
    }
    fs::write(&index_path, &index)?;
    fs::write(&blob_path, &blob)?;
    fs::write(&played_path, &played)?;
    let header = json!({
        "version": VERSION,
        "rule": COARSE_CLASS_RULE,
        "classes": COARSE_CLASSES,
        "frames": "every labelled priority frame of a recorded seat",
        "key": ["game", "seat", "cursor"],
        "index": {"file": "coarse-classes.index.bin", "row_bytes": ROW_BYTES,
            "sorted_by": ["game", "seat", "cursor"],
            "dtype": [["game", "<u4"], ["cursor", "<u4"], ["seat", "u1"], ["r_class", "u1"],
                ["n_class", "u1"], ["subtype", "u1"], ["r_idx", "<u4"], ["n_idx", "<u4"],
                ["n_cand", "<u4"], ["offset", "<u8"]],
            "sha256": sha256(&index)},
        "played_file": {"file": "coarse-classes.played.bin", "row_bytes": PLAYED_ROW_BYTES,
            "rows": "one per index row, in its order",
            "dtype": [["p_idx", "<u4"], ["p_class", "u1"], ["free", "u1"], ["pad", "V2"]],
            "p_idx": "the record's answer (what the seat played), u32::MAX where none",
            "free": "1 where nothing was owed (subtype not pay): a free priority",
            "sha256": sha256(&played)},
        "classes_file": {"file": "coarse-classes.classes.bin", "bytes": blob.len(),
            "sha256": sha256(&blob)},
        "subtype_codes": DistillSubtype4::ALL.map(DistillSubtype4::name),
        "rows": rows.len(),
        "counts": {"n_ne_r": n_ne_r, "n_eq_r": n_eq_r, "r_class_by_subtype": by_subtype,
            "free_rows": free_rows, "at_free": at_free},
        "source": extra,
    });
    fs::write(&header_path, serde_json::to_vec_pretty(&header)?)
        .with_context(|| format!("writing {}", header_path.display()))?;
    Ok(header)
}

fn sha256(bytes: &[u8]) -> String {
    super::shadow::sha(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(game: u32, seat: u8, cursor: u32, classes: &[u8], r: u32, n: u32) -> Row {
        Row {
            game,
            cursor,
            seat,
            subtype: DistillSubtype4::Mana.code(),
            r_idx: r,
            n_idx: n,
            // The layer played N where it is a mana answer, else nothing.
            p_idx: if classes.get(n as usize) == Some(&1) {
                n
            } else {
                u32::MAX
            },
            free: game == 1,
            n_ne_r: r != n,
            classes: classes.to_vec(),
        }
    }

    #[test]
    fn the_files_hold_sorted_rows_their_classes_and_the_counts() {
        let dir = std::env::temp_dir().join(format!("coarse-classes-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        // Out of order on purpose: a later game first.
        let rows = vec![
            row(2, 0, 5, &[0, 1, 2], 1, 0),
            row(1, 1, 3, &[0, 1, 1, 3], 2, 2),
        ];
        let header = write(&dir, rows, &json!({"runs": ["r"]})).unwrap();
        let index = fs::read(dir.join("coarse-classes.index.bin")).unwrap();
        let blob = fs::read(dir.join("coarse-classes.classes.bin")).unwrap();
        let played = fs::read(dir.join("coarse-classes.played.bin")).unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(index.len(), 2 * ROW_BYTES);
        assert_eq!(blob, [0, 1, 1, 3, 0, 1, 2]);
        let word = |row: usize, at: usize| {
            u32::from_le_bytes(
                index[row * ROW_BYTES + at..row * ROW_BYTES + at + 4]
                    .try_into()
                    .unwrap(),
            )
        };
        // The first row is game 1: cursor 3, seat 1, R tap, N tap, 4
        // candidates at offset 0.
        assert_eq!((word(0, 0), word(0, 4)), (1, 3));
        assert_eq!(&index[8..12], &[1, 1, 1, DistillSubtype4::Mana.code()]);
        assert_eq!((word(0, 12), word(0, 16), word(0, 20)), (2, 2, 4));
        // The second: game 2, R tap where N passes, its classes at 4.
        assert_eq!(&index[ROW_BYTES + 8..ROW_BYTES + 12], &[0, 1, 0, 5]);
        assert_eq!(
            u64::from_le_bytes(index[ROW_BYTES + 24..ROW_BYTES + 32].try_into().unwrap()),
            4
        );
        assert_eq!(header["rows"], 2);
        assert_eq!(header["rule"], "coarse_class_v1");
        assert_eq!(header["counts"]["n_ne_r"]["R tap / N pass"], 1);
        assert_eq!(header["counts"]["n_eq_r"]["R tap / N tap"], 1);
        assert_eq!(header["counts"]["r_class_by_subtype"]["mana"]["tap"], 2);
        // Version 2: the played answers, one row per index row in its order.
        assert_eq!(header["version"], 2);
        assert_eq!(played.len(), 2 * PLAYED_ROW_BYTES);
        assert_eq!(played[..PLAYED_ROW_BYTES], [2, 0, 0, 0, 1, 1, 0, 0]);
        assert_eq!(
            played[PLAYED_ROW_BYTES..],
            [255, 255, 255, 255, NO_CLASS, 0, 0, 0]
        );
        assert_eq!(header["played_file"]["row_bytes"], 8);
        assert_eq!(header["counts"]["free_rows"], 1);
        assert_eq!(header["counts"]["at_free"]["p"]["tap"], 1);
        assert_eq!(header["counts"]["at_free"]["r"]["tap"], 1);
    }
}
