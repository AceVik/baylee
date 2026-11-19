//! The probe sidecar of a distillation dataset (`distill_probe_v1`): what
//! the tactical layer's answer `R` was worth beside the net's `N` at each
//! probed frame, played on in the same fair worlds (`eval4 --search-rival
//! distill`). Beside the frames, never in them, keyed by (game, seat,
//! cursor) as the coarse-class sidecar is.
//!
//! - `probe.index.bin`: one 56-byte little-endian row per probed frame,
//!   sorted by game, seat and cursor ([`ROW_BYTES`], `dtype` in the header):
//!   game `u32`, cursor `u32`, seat `u8`, the label's subtype code `u8`, the
//!   worlds `u16`, `N`'s and `R`'s menu entries `u16` each, the mask of the
//!   groups that fired `u64`, then `f32`s: `R`'s mean fork advantage over
//!   `N`, its standard error over the worlds (`NaN` with one), the mean in
//!   the even and in the odd worlds (`NaN` for an empty half), `N`'s and `R`'s
//!   mean scores; then `u16`s: rollouts that ended the game and rollouts
//!   that stopped short (`stopped`, `house_stop`, `taken_back`), and the
//!   house answers in them `u32`;
//! - `probe.json`: the header: the layout, the files' SHA-256 and what the
//!   probes say by subtype and by group.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context as _, ensure};
use baylee_train::convert4::Step4;
use baylee_train::convert4::distill_labels::{
    DISTILL_PROBE_CONTRACT, DistillFrame4, DistillSubtype4, FIRED_GROUPS,
};
use serde_json::{Value, json};

/// The bytes of one row.
pub(super) const ROW_BYTES: usize = 56;

/// The layout's version.
pub(super) const VERSION: u32 = 1;

/// One probed frame's summary.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Row {
    game: u32,
    cursor: u32,
    seat: u8,
    subtype: u8,
    worlds: u16,
    n_idx: u16,
    r_idx: u16,
    fired: u64,
    mean: f32,
    se: f32,
    halves: [f32; 2],
    mean_n: f32,
    mean_r: f32,
    terminal: u16,
    stops: u16,
    house_answers: u32,
}

/// A count as a float, for means.
fn real(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "Scores are utilities in [0, 1]; f32 keeps them"
)]
fn narrow(x: f64) -> f32 {
    x as f32
}

impl Row {
    /// The row of a frame, when it is a labelled frame with a probe.
    pub(super) fn of(step: &Step4) -> Option<Self> {
        Self::of_frame(
            step.game_id,
            step.step_cursor,
            step.seat,
            step.distill.as_ref()?,
        )
    }

    /// The row of game `game`'s frame at `cursor` of `seat`, `distill` its
    /// distillation block, when it is labelled and probed.
    fn of_frame(game: u32, cursor: u32, seat: u8, distill: &DistillFrame4) -> Option<Self> {
        let label = distill.label.as_ref()?;
        let probe = label.probe.as_ref()?;
        let (mean, se) = probe.advantage()?;
        let n = real(probe.worlds.len());
        let column = |at: usize| probe.worlds.iter().map(|w| f64::from(w[at])).sum::<f64>() / n;
        let count = |names: &[&str]| {
            let ends = probe
                .ends
                .iter()
                .flatten()
                .filter(|end| names.contains(&end.as_str()))
                .count();
            u16::try_from(ends).unwrap_or(u16::MAX)
        };
        let [even, odd] = probe.halves();
        Some(Self {
            game,
            cursor,
            seat,
            subtype: distill.subtype_code,
            worlds: u16::try_from(probe.worlds.len()).unwrap_or(u16::MAX),
            n_idx: probe.own_index,
            r_idx: probe.rival_index,
            fired: distill.fired_mask,
            mean: narrow(mean),
            se: se.map_or(f32::NAN, narrow),
            halves: [even.map_or(f32::NAN, narrow), odd.map_or(f32::NAN, narrow)],
            mean_n: narrow(column(0)),
            mean_r: narrow(column(1)),
            terminal: count(&["terminal"]),
            stops: count(&["stopped", "house_stop", "taken_back"]),
            house_answers: u32::try_from(probe.house_answers).unwrap_or(u32::MAX),
        })
    }

    fn bytes(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.game.to_le_bytes());
        out.extend_from_slice(&self.cursor.to_le_bytes());
        out.extend_from_slice(&[self.seat, self.subtype]);
        out.extend_from_slice(&self.worlds.to_le_bytes());
        out.extend_from_slice(&self.n_idx.to_le_bytes());
        out.extend_from_slice(&self.r_idx.to_le_bytes());
        out.extend_from_slice(&self.fired.to_le_bytes());
        for x in [
            self.mean,
            self.se,
            self.halves[0],
            self.halves[1],
            self.mean_n,
            self.mean_r,
        ] {
            out.extend_from_slice(&x.to_le_bytes());
        }
        out.extend_from_slice(&self.terminal.to_le_bytes());
        out.extend_from_slice(&self.stops.to_le_bytes());
        out.extend_from_slice(&self.house_answers.to_le_bytes());
    }

    /// Whether `R` is ahead of `N` by more than one standard error.
    fn r_ahead(&self) -> bool {
        self.se.is_finite() && self.mean - self.se > 0.0
    }

    /// Whether `N` is ahead of `R` by more than one standard error.
    fn n_ahead(&self) -> bool {
        self.se.is_finite() && self.mean + self.se < 0.0
    }
}

/// What the probes of one subtype or group say.
#[derive(Clone, Debug, Default, serde::Serialize)]
struct Tally {
    rows: u64,
    /// The mean of the rows' mean advantages.
    mean: f64,
    /// Rows where `R` leads by more than one standard error.
    r_ahead: u64,
    /// Rows where `N` leads by more than one standard error.
    n_ahead: u64,
    #[serde(skip)]
    sum: f64,
}

impl Tally {
    fn add(&mut self, row: &Row) {
        self.rows += 1;
        self.sum += f64::from(row.mean);
        self.mean = self.sum / real(usize::try_from(self.rows).unwrap_or(usize::MAX));
        self.r_ahead += u64::from(row.r_ahead());
        self.n_ahead += u64::from(row.n_ahead());
    }
}

fn subtype_name(code: u8) -> &'static str {
    DistillSubtype4::ALL
        .iter()
        .find(|s| s.code() == code)
        .map_or("none", |s| s.name())
}

/// Writes the files under `dir` and returns the header.
///
/// # Errors
/// When a file exists already or cannot be written.
pub(super) fn write(dir: &Path, mut rows: Vec<Row>, extra: &Value) -> anyhow::Result<Value> {
    fs::create_dir_all(dir)?;
    let index_path = dir.join("probe.index.bin");
    let header_path = dir.join("probe.json");
    for path in [&index_path, &header_path] {
        ensure!(!path.exists(), "{} exists", path.display());
    }
    rows.sort_by_key(|row| (row.game, row.seat, row.cursor));
    let mut index = Vec::with_capacity(rows.len() * ROW_BYTES);
    let mut all = Tally::default();
    let mut by_subtype: BTreeMap<&str, Tally> = BTreeMap::new();
    let mut by_group: BTreeMap<&str, Tally> = BTreeMap::new();
    for row in &rows {
        row.bytes(&mut index);
        all.add(row);
        by_subtype
            .entry(subtype_name(row.subtype))
            .or_default()
            .add(row);
        let mut any = false;
        for (bit, group) in FIRED_GROUPS.iter().enumerate() {
            if row.fired & (1 << bit) != 0 {
                by_group.entry(group).or_default().add(row);
                any = true;
            }
        }
        if !any {
            by_group.entry("none").or_default().add(row);
        }
    }
    fs::write(&index_path, &index)?;
    let header = json!({
        "version": VERSION,
        "contract": DISTILL_PROBE_CONTRACT,
        "frames": "every labelled frame whose N and R were probed in the same fair worlds",
        "key": ["game", "seat", "cursor"],
        "advantage": "R's score less N's in the same world, averaged over the worlds",
        "index": {"file": "probe.index.bin", "row_bytes": ROW_BYTES,
            "sorted_by": ["game", "seat", "cursor"],
            "dtype": [["game", "<u4"], ["cursor", "<u4"], ["seat", "u1"], ["subtype", "u1"],
                ["worlds", "<u2"], ["n_idx", "<u2"], ["r_idx", "<u2"], ["fired", "<u8"],
                ["mean", "<f4"], ["se", "<f4"], ["half_even", "<f4"], ["half_odd", "<f4"],
                ["mean_n", "<f4"], ["mean_r", "<f4"], ["terminal", "<u2"], ["stops", "<u2"],
                ["house_answers", "<u4"]],
            "nan": "se with one world, a half with no world",
            "sha256": super::shadow::sha(&index)},
        "subtype_codes": DistillSubtype4::ALL.map(DistillSubtype4::name),
        "groups": FIRED_GROUPS,
        "rows": rows.len(),
        "counts": {"all": all, "by_subtype": by_subtype, "by_group": by_group,
            "r_ahead": "mean - se > 0", "n_ahead": "mean + se < 0"},
        "source": extra,
    });
    fs::write(&header_path, serde_json::to_vec_pretty(&header)?)
        .with_context(|| format!("writing {}", header_path.display()))?;
    Ok(header)
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_train::convert4::distill_labels::{DistillFrameLabel4, DistillProbe4};

    fn probe(worlds: &[[f32; 2]]) -> DistillProbe4 {
        DistillProbe4 {
            rival_index: 3,
            own_index: 0,
            worlds: worlds.to_vec(),
            ends: worlds
                .iter()
                .enumerate()
                .map(|(p, _)| {
                    if p == 0 {
                        ["terminal".to_owned(), "house_stop".to_owned()]
                    } else {
                        ["boundary".to_owned(), "boundary".to_owned()]
                    }
                })
                .collect(),
            rollouts: 2 * worlds.len() as u64,
            house_answers: 0,
        }
    }

    fn row(game: u32, cursor: u32, probe: Option<DistillProbe4>) -> Option<Row> {
        let frame = DistillFrame4 {
            subtype_code: DistillSubtype4::Spend.code(),
            fired_mask: 1 << FIRED_GROUPS.iter().position(|g| *g == "react").unwrap(),
            label: Some(DistillFrameLabel4 {
                label_kind: "priority".into(),
                n_ne_r: true,
                net_answer: baylee_train::convert4::NetAnswer4::default(),
                subtype: Some(DistillSubtype4::Spend),
                fired: vec!["react".into()],
                played: Some(0),
                probe,
            }),
        };
        Row::of_frame(game, cursor, 1, &frame)
    }

    #[test]
    fn a_probed_frame_is_one_sorted_row_and_the_header_tallies_it() {
        assert_eq!(row(1, 4, None), None, "no probe, no row");
        let ahead = row(2, 9, Some(probe(&[[0.4, 0.6], [0.5, 0.6], [0.5, 0.7]]))).unwrap();
        let behind = row(1, 4, Some(probe(&[[0.6, 0.4], [0.7, 0.5]]))).unwrap();
        assert!((ahead.mean - 0.1667).abs() < 1e-3, "{ahead:?}");
        assert!(ahead.r_ahead() && !ahead.n_ahead());
        assert!(behind.n_ahead());
        assert_eq!((ahead.terminal, ahead.stops), (1, 1));
        // Worlds 0 and 2 against world 1.
        assert!((ahead.halves[0] - 0.2).abs() < 1e-6, "{ahead:?}");
        assert!((ahead.halves[1] - 0.1).abs() < 1e-6, "{ahead:?}");

        let dir = std::env::temp_dir().join(format!("probes-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let header = write(&dir, vec![ahead, behind], &json!({"runs": ["r"]})).unwrap();
        let index = fs::read(dir.join("probe.index.bin")).unwrap();
        let _ = fs::remove_dir_all(&dir);
        assert_eq!(index.len(), 2 * ROW_BYTES);
        let word = |row: usize, at: usize| {
            u32::from_le_bytes(
                index[row * ROW_BYTES + at..row * ROW_BYTES + at + 4]
                    .try_into()
                    .unwrap(),
            )
        };
        let real = |row: usize, at: usize| f32::from_bits(word(row, at));
        // Sorted: game 1 first.
        assert_eq!((word(0, 0), word(0, 4)), (1, 4));
        assert_eq!(index[8], 1);
        assert_eq!(index[9], DistillSubtype4::Spend.code());
        assert_eq!(u16::from_le_bytes([index[10], index[11]]), 2);
        assert_eq!(u16::from_le_bytes([index[14], index[15]]), 3);
        assert!((real(0, 24) + 0.2).abs() < 1e-6);
        assert!((real(1, 40) - 0.4667).abs() < 1e-3, "N's mean score");
        assert_eq!(header["rows"], 2);
        assert_eq!(header["counts"]["all"]["r_ahead"], 1);
        assert_eq!(header["counts"]["all"]["n_ahead"], 1);
        assert_eq!(header["counts"]["by_subtype"]["spend"]["rows"], 2);
        assert_eq!(header["counts"]["by_group"]["react"]["rows"], 2);
        assert_eq!(header["index"]["row_bytes"], ROW_BYTES);
    }
}
