//! Search afterstates (`--search-afterstates`, `search_afterstate_v1`): beside
//! the search-advantage labels, the position each searched candidate leads to
//! (`baylee_train::convert4::afterstates`), as the seat observes it.
//!
//! Each labelled root's searched candidates get one row each in the sidecar
//! `search-afterstates/`; a candidate whose afterstate is valid also gets its
//! frame in the dataset's split `search-afterstate-frames`, an observation frame
//! (no candidates, policy weight 0) under the root's game and seat at cursor
//! `8 * root cursor + candidate`. An afterstate is valid where the fork
//! reached a position, the fork's frame of the root is the replay's byte for
//! byte, and both forks' frames of the afterstate are the same bytes.
//!
//! - `index.bin`: one 48-byte little-endian row per searched candidate of a
//!   labelled root, sorted by game, seat, root cursor and candidate: game
//!   `u32`, root cursor `u32`, the candidate's item on the root's payable
//!   menu `u32` (the sidecar's), the frame's cursor `u32` (`u32::MAX` without
//!   a frame), seat `u8`, candidate `u8` (its place in the event, the policy's
//!   own 0), stop `u8` ([`STOPS`]), flags `u8` ([`FLAGS`]), answers applied
//!   `u16`, the next question's `PendingKind4` `u8` (255 none), worlds `u8`,
//!   then `f32` each: the mean and the standard error of the candidate's
//!   world-by-world lead over the policy's own on the even worlds and on the
//!   odd ones (NaN where undefined), the card the candidate casts `u32`
//!   (`u32::MAX` none), four zero bytes;
//! - `header.json`: the contract, the layout, the digest and the counts.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::ensure;
use baylee_engine::choice::PlayerAction;
use baylee_train::convert4::afterstates::{AfterStop4, Afterstate4, RootAfterstates4};
use baylee_train::convert4::{AuxTargets4, Step4};
use baylee_train::features4::{PendingKind4, SourceKind4, StepProvenance4, TeacherTarget4};
use serde_json::{Value, json};

/// The dataset contract.
pub(super) const AFTERSTATE_CONTRACT: &str = "search_afterstate_v1";
/// The split of the afterstate frames, which the trajectory cache reads.
pub(super) const AFTER_SPLIT: &str = baylee_train_ops::search_advantage::AFTERSTATE_FRAMES_SPLIT;
/// The `meta.json` key that lists their shards.
pub(super) const AFTER_SHARDS_KEY: &str = baylee_train_ops::search_advantage::AFTERSTATE_SHARDS_KEY;
/// The bytes of an index row.
pub(super) const AFTER_ROW: usize = 48;
/// Candidates per root a frame cursor has room for.
const PER_ROOT: u32 = baylee_train_ops::search_advantage::AFTER_PER_ROOT;

/// The stop codes: [`AfterStop4`]'s, then the two checks a reached position
/// can fail.
pub(super) const STOPS: [&str; 9] = [
    "question",
    "game_over",
    "fork_refused",
    "answer_refused",
    "script_mismatch",
    "script_broken",
    "too_long",
    "root_mismatch",
    "forks_differ",
];
/// The flag bits, lowest first.
pub(super) const FLAGS: [&str; 6] = [
    "valid",
    "root_match",
    "forks_match",
    "hidden_event",
    "asked_seat",
    "game_over",
];

/// One searched candidate's row.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct AfterRow {
    game: u32,
    root_cursor: u32,
    item: u32,
    frame_cursor: u32,
    seat: u8,
    candidate: u8,
    stop: u8,
    flags: u8,
    answers: u16,
    next_kind: u8,
    worlds: u8,
    halves: [f32; 4],
    card: u32,
}

/// What became of the afterstates, by reason.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub(super) struct AfterCounts {
    roots: u64,
    candidates: u64,
    valid: u64,
    roots_own_valid: u64,
    by_stop: BTreeMap<String, u64>,
    /// Valid afterstates whose first fork saw a hidden-information event
    /// (both forks still gave the same frame).
    hidden_event_valid: u64,
    /// Candidates whose forks differ and whose first fork saw one.
    hidden_event_differ: u64,
    /// Valid afterstates whose two forks' engine states (hidden cards
    /// included) differ: the deals really were different.
    deal_differs_valid: u64,
    /// Roots whose fork frame of the root differs from the replay's.
    root_mismatch_roots: u64,
}

impl AfterCounts {
    pub(super) fn add(&mut self, other: &Self) {
        self.roots += other.roots;
        self.candidates += other.candidates;
        self.valid += other.valid;
        self.roots_own_valid += other.roots_own_valid;
        for (key, n) in &other.by_stop {
            *self.by_stop.entry(key.clone()).or_default() += n;
        }
        self.hidden_event_valid += other.hidden_event_valid;
        self.hidden_event_differ += other.hidden_event_differ;
        self.deal_differs_valid += other.deal_differs_valid;
        self.root_mismatch_roots += other.root_mismatch_roots;
    }
}

/// A count as a float, for means.
fn real(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// The mean and standard error of `candidate`'s lead over the policy's own
/// answer (candidate 0) on the worlds of parity `parity` (0 even, 1 odd);
/// the standard error over `n - 1`, then over `n` for the mean; NaN where
/// there are too few worlds.
pub(super) fn half(worlds: &[Vec<f64>], candidate: usize, parity: usize) -> (f64, f64) {
    let leads: Vec<f64> = worlds
        .iter()
        .skip(parity)
        .step_by(2)
        .filter_map(|world| Some(world.get(candidate)? - world.first()?))
        .collect();
    if leads.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    let n = real(leads.len());
    let mean = leads.iter().sum::<f64>() / n;
    if leads.len() < 2 {
        return (mean, f64::NAN);
    }
    let variance = leads.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1.0);
    (mean, (variance / n).sqrt())
}

/// The card a candidate casts: its own cast, or the cast its taps pay for.
fn cast_card(action: &PlayerAction, script: Option<&PlayerAction>) -> u32 {
    let card = |action: &PlayerAction| match action {
        PlayerAction::CastSpell { card } => serde_json::to_value(card)
            .ok()
            .and_then(|value| value.as_u64())
            .and_then(|id| u32::try_from(id).ok()),
        _ => None,
    };
    script
        .and_then(card)
        .or_else(|| card(action))
        .unwrap_or(u32::MAX)
}

/// An afterstate's observation frame, as the replay writes one.
fn frame(
    root: &Step4,
    cursor: u32,
    state: baylee_train::features4::EncodedState4,
    turn: u32,
) -> Step4 {
    Step4 {
        game_id: root.game_id,
        seat: root.seat,
        step_cursor: cursor,
        turn: u16::try_from(turn).unwrap_or(u16::MAX),
        is_decision: false,
        loss_weight: 0.0,
        is_terminal: false,
        state,
        head: 0,
        pending_kind: PendingKind4::Priority as u8,
        cand_feat: Vec::new(),
        cand_entity_idx: Vec::new(),
        cand_rel_seat_idx: Vec::new(),
        cand_card_id: Vec::new(),
        cand_face_id: Vec::new(),
        cand_ability_idx: Vec::new(),
        cand_rule_refs: Vec::new(),
        pairs: Vec::new(),
        target: TeacherTarget4::default(),
        outcome: 0.0,
        aux: AuxTargets4::default(),
        provenance: StepProvenance4 {
            source: SourceKind4::Forced,
            ..StepProvenance4::default()
        },
        shadow_label: None,
        llm_label: None,
        seat_input: None,
        distill: None,
        cand_classes: None,
        behavior_mixture: None,
        search_root: None,
    }
}

/// The root frame as the first fork encodes it: the replay's frame with the
/// fork's state and menu.
fn fork_root(
    step: &Step4,
    state: &baylee_train::features4::EncodedState4,
    cands: &baylee_train::features4::EncodedCandidates4,
) -> Step4 {
    let mut out = step.clone();
    out.state = state.clone();
    out.cand_rule_refs = cands
        .candidates
        .iter()
        .map(|c| baylee_train::cardstructure::refs::candidate_refs(c, state))
        .collect();
    out.cand_feat = cands.candidates.iter().map(|c| c.feat.clone()).collect();
    out.cand_entity_idx = cands.candidates.iter().map(|c| c.entity_idx).collect();
    out.cand_rel_seat_idx = cands.candidates.iter().map(|c| c.rel_seat_idx).collect();
    out.cand_card_id = cands.candidates.iter().map(|c| c.card_id).collect();
    out.cand_face_id = cands
        .candidates
        .iter()
        .map(baylee_train::features4::Candidate4::named_face)
        .collect();
    out.cand_ability_idx = cands.candidates.iter().map(|c| c.ability_idx).collect();
    out.pairs.clone_from(&cands.pairs);
    out
}

/// One game's afterstates: its frames' stream, rows and counts.
pub(super) struct GameAfterstates {
    writer: super::output::GameWriter,
    rows: Vec<AfterRow>,
    counts: AfterCounts,
}

/// What a labelled root's event says of its candidates.
pub(super) struct EventCandidates<'a> {
    pub(super) candidates: &'a [PlayerAction],
    pub(super) scripts: &'a [Option<PlayerAction>],
    pub(super) worlds: &'a [Vec<f64>],
    /// Each candidate's item on the root's payable menu.
    pub(super) items: &'a [usize],
}

impl GameAfterstates {
    /// Starts a game's afterstates in the dataset `root`'s afterstate split.
    ///
    /// # Errors
    /// When the stream cannot be opened.
    pub(super) fn new(
        root: &Path,
        game_id: u32,
        rule_roots: std::sync::Arc<BTreeMap<String, u32>>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            writer: super::output::GameWriter::in_split(root, AFTER_SPLIT, game_id, rule_roots)?,
            rows: Vec::new(),
            counts: AfterCounts::default(),
        })
    }

    /// Checks and writes a labelled root's afterstates: `step` is the root's
    /// frame as the replay wrote it, `captured` what its forks gave.
    ///
    /// # Errors
    /// A root with more candidates than a cursor has room for, or a frame
    /// that cannot be packed or written.
    pub(super) fn record(
        &mut self,
        step: &Step4,
        captured: &RootAfterstates4,
        event: &EventCandidates<'_>,
    ) -> Result<(), String> {
        let n = event.candidates.len();
        if n > PER_ROOT as usize || captured.candidates.len() != n || event.items.len() != n {
            return Err(format!(
                "root at cursor {} has {n} candidates, {} afterstates and {} items",
                step.step_cursor,
                captured.candidates.len(),
                event.items.len()
            ));
        }
        let own_bytes = self.writer.payload(step).map_err(|e| e.to_string())?;
        let root_match = match &captured.root {
            Some((state, cands)) => {
                self.writer
                    .payload(&fork_root(step, state, cands))
                    .map_err(|e| e.to_string())?
                    == own_bytes
            }
            None => false,
        };
        self.counts.roots += 1;
        if !root_match {
            self.counts.root_mismatch_roots += 1;
        }
        for (candidate, after) in captured.candidates.iter().enumerate() {
            let row = self.candidate(step, root_match, candidate, after, event)?;
            if candidate == 0 && row.flags & 1 == 1 {
                self.counts.roots_own_valid += 1;
            }
            self.rows.push(row);
        }
        Ok(())
    }

    fn candidate(
        &mut self,
        step: &Step4,
        root_match: bool,
        candidate: usize,
        after: &Afterstate4,
        event: &EventCandidates<'_>,
    ) -> Result<AfterRow, String> {
        let at = u32::try_from(candidate).map_err(|e| e.to_string())?;
        let cursor = step.step_cursor * PER_ROOT + at;
        let frames = match &after.frames {
            [Some(first), Some(second)] if after.stop.reached() => Some((
                frame(step, cursor, first.state.clone(), first.turn),
                frame(step, cursor, second.state.clone(), second.turn),
            )),
            _ => None,
        };
        let forks_match = match &frames {
            Some((first, second)) => {
                self.writer.payload(first).map_err(|e| e.to_string())?
                    == self.writer.payload(second).map_err(|e| e.to_string())?
            }
            None => false,
        };
        let stop = if !after.stop.reached() {
            after.stop as u8
        } else if !root_match {
            7
        } else if !forks_match {
            8
        } else {
            after.stop as u8
        };
        let valid = stop <= AfterStop4::GameOver as u8;
        self.counts.candidates += 1;
        *self
            .counts
            .by_stop
            .entry(STOPS[usize::from(stop)].to_owned())
            .or_default() += 1;
        if valid {
            self.counts.valid += 1;
            self.counts.hidden_event_valid += u64::from(after.hidden_event);
            if let [Some(first), Some(second)] = &after.frames {
                self.counts.deal_differs_valid += u64::from(first.hash != second.hash);
            }
        } else if stop == 8 {
            self.counts.hidden_event_differ += u64::from(after.hidden_event);
        }
        let frame_cursor = match (&frames, valid) {
            (Some((first, _)), true) => {
                self.writer.push(first).map_err(|e| e.to_string())?;
                cursor
            }
            _ => u32::MAX,
        };
        let flags = u8::from(valid)
            | (u8::from(root_match) << 1)
            | (u8::from(forks_match) << 2)
            | (u8::from(after.hidden_event) << 3)
            | (u8::from(after.asked_seat) << 4)
            | (u8::from(after.stop == AfterStop4::GameOver) << 5);
        #[allow(clippy::cast_possible_truncation, reason = "statistics are f32")]
        let halves = {
            let (even, even_se) = half(event.worlds, candidate, 0);
            let (odd, odd_se) = half(event.worlds, candidate, 1);
            [even as f32, even_se as f32, odd as f32, odd_se as f32]
        };
        Ok(AfterRow {
            game: step.game_id,
            root_cursor: step.step_cursor,
            item: u32::try_from(event.items[candidate]).map_err(|e| e.to_string())?,
            frame_cursor,
            seat: step.seat,
            candidate: u8::try_from(candidate).map_err(|e| e.to_string())?,
            stop,
            flags,
            answers: after.answers,
            next_kind: after.next_kind,
            worlds: u8::try_from(event.worlds.len()).unwrap_or(u8::MAX),
            halves,
            card: cast_card(
                &event.candidates[candidate],
                event.scripts.get(candidate).and_then(Option::as_ref),
            ),
        })
    }

    /// Publishes the game's frames and gives its rows, counts and shard.
    ///
    /// # Errors
    /// When the stream cannot be published.
    pub(super) fn finish(self) -> anyhow::Result<(Vec<AfterRow>, AfterCounts, String)> {
        let shard = self.writer.finish()?;
        Ok((self.rows, self.counts, shard))
    }
}

/// Writes the sidecar `dir` (`index.bin`, `header.json`) and gives its header.
///
/// # Errors
/// When `dir` already holds a sidecar or a file cannot be written.
pub(super) fn write(
    dir: &Path,
    mut rows: Vec<AfterRow>,
    counts: &AfterCounts,
    extra: &Value,
) -> anyhow::Result<Value> {
    fs::create_dir_all(dir)?;
    let index_path = dir.join("index.bin");
    let header_path = dir.join("header.json");
    ensure!(
        !index_path.exists() && !header_path.exists(),
        "{} holds a sidecar",
        dir.display()
    );
    rows.sort_by_key(|row| (row.game, row.seat, row.root_cursor, row.candidate));
    let mut index = Vec::with_capacity(rows.len() * AFTER_ROW);
    for row in &rows {
        for word in [row.game, row.root_cursor, row.item, row.frame_cursor] {
            index.extend_from_slice(&word.to_le_bytes());
        }
        index.extend_from_slice(&[row.seat, row.candidate, row.stop, row.flags]);
        index.extend_from_slice(&row.answers.to_le_bytes());
        index.extend_from_slice(&[row.next_kind, row.worlds]);
        for value in row.halves {
            index.extend_from_slice(&value.to_le_bytes());
        }
        index.extend_from_slice(&row.card.to_le_bytes());
        index.extend_from_slice(&[0; 4]);
    }
    fs::write(&index_path, &index)?;
    let header = json!({
        "contract": AFTERSTATE_CONTRACT,
        "version": 1,
        "key": ["game", "seat", "root_cursor", "candidate"],
        "frames": {"split": AFTER_SPLIT, "meta_key": AFTER_SHARDS_KEY,
            "cursor": "8 * root cursor + candidate, under the root's game and seat",
            "frame": "the seat's observation (encoder 10, observe_view_with_context, or its \
                      question's view where it owes the next question), knowledge observed \
                      once from the root's; an observation frame, policy weight 0"},
        "index": {"file": "index.bin", "row_bytes": AFTER_ROW, "rows": rows.len(),
            "sorted_by": ["game", "seat", "root_cursor", "candidate"],
            "dtype": [["game", "<u4"], ["root_cursor", "<u4"], ["item", "<u4"],
                ["frame_cursor", "<u4"], ["seat", "u1"], ["candidate", "u1"], ["stop", "u1"],
                ["flags", "u1"], ["answers", "<u2"], ["next_kind", "u1"], ["worlds", "u1"],
                ["even_mean", "<f4"], ["even_se", "<f4"], ["odd_mean", "<f4"],
                ["odd_se", "<f4"], ["card", "<u4"], ["pad", "V4"]],
            "stops": STOPS, "flags": FLAGS, "none": u32::MAX,
            "halves": "world-by-world lead over the policy's own answer (candidate 0) on the \
                       even worlds (0, 2) and the odd ones (1, 3): mean and standard error \
                       (variance over n - 1, then over n); NaN where undefined",
            "card": "the object id the candidate casts (its own cast or its planned cast)",
            "sha256": super::shadow::sha(&index)},
        "rule": "baylee_train::convert4::afterstates: forks under DeckKnowledge::Prior from two \
                 seeds of (game, seat, root cursor), every chair driven; the candidate as the \
                 search played it (a planned cast's Script4); stop at the next question; valid \
                 only where the fork's root frame is the replay's and both forks' afterstate \
                 frames are the same bytes",
        "counts": counts,
        "source": extra,
    });
    fs::write(&header_path, serde_json::to_vec_pretty(&header)?)?;
    Ok(header)
}

#[cfg(test)]
mod tests;
