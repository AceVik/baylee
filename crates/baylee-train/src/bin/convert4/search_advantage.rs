//! Search-advantage labels (`--search-advantage`, `search_advantage_v1`):
//! at each searched priority root of a recorded seat, the fork advantage of
//! every answer the search tried over the policy's own, with its standard
//! error, on the items of the root's payable menu, for a student that reads
//! that menu in a second pass at the roots (`baylee_train::search_labels`,
//! `netplay4::menu::PayableMenu4`).
//!
//! The frames are the record's, every one at policy weight 0: no answer the
//! search played becomes a policy target. A labelled root has a twin, its
//! frame again with the payable menu, in the split `search-advantage-menus`
//! under the root's own (game, seat, cursor), ending in its BSA1 block
//! (`baylee_train_ops::search_advantage`). The labels are the sidecar
//! `search-advantage/`:
//!
//! - `index.bin`: one 48-byte little-endian row per labelled root, sorted by
//!   game, seat and cursor: game `u32`, cursor `u32` (the twin's, which is
//!   the root's), root cursor `u32`, seat `u8`, root kind `u8`
//!   ([`ROOT_KINDS`]), two zero bytes, the policy's own item `u32`, the
//!   number of items `u32`, the search's choice over all its worlds `u32`,
//!   its choices on worlds {0, 2} and {1, 3} `u32` (`search_labels::halves`,
//!   margin 0.03; `u32::MAX` none), the offset of its items in `cands.bin`
//!   `u64`, four zero bytes;
//! - `cands.bin`: one 12-byte row per item of each root's payable menu, in
//!   menu order: the advantage `f32` and its standard error `f32` over the
//!   policy's own answer (`search_labels::paired`; NaN where the item was no
//!   candidate; 0 and 0 at the policy's own item), searched `u8`, added by
//!   the payable menu `u8`, coarse class `u8` (`coarse_class_v1`), a zero
//!   byte;
//! - `header.json`: the contract, the layouts, the files' SHA-256 and the
//!   counts, with every root left out by its reason.
//!
//! A root is labelled only where every one of these holds, and otherwise
//! counted by the reason it fails: the event was accepted; no rollout of it
//! was stopped (`leaves.stopped == 0`); the game had no fallback or refused
//! answer; the root is a priority question; every candidate takes an item of
//! its own (`search_labels::macro_items`); and the event's own advantages and
//! standard errors are the ones its worlds give. An event that does not bind
//! to its frame (its played answer is not the record's) rejects the game.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context as _, ensure};
use baylee_engine::choice::PlayerAction;
use baylee_train::convert4::Step4;
use baylee_train::convert4::search_roots::{AfterCandidate4, Roots4, SearchRoot4};
use baylee_train::features4::EncodedCandidates4;
use baylee_train::search_labels::{HALVES_MARGIN, halves, macro_item, macro_items, paired};
use baylee_train_ops::search_advantage::{
    MENUS_SHARDS_KEY, MENUS_SPLIT, SEARCH_ADVANTAGE_CONTRACT, twin_block,
};
use baylee_view::PlayerView;
use serde::Deserialize;
use serde_json::{Value, json};

/// The bytes of an index row.
pub(super) const INDEX_ROW: usize = 48;
/// The bytes of an item row.
pub(super) const CAND_ROW: usize = 12;

/// The root kinds, by code: the policy's own answer at the root.
pub(super) const ROOT_KINDS: [&str; 5] = ["pass", "tap", "land", "activate", "cast"];

/// Two numbers that should be one, read back from an event's `f32`s.
const BIND: f64 = 1e-4;

/// How far an event's own numbers may sit from the ones its worlds give.
fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= BIND * (1.0 + a.abs().max(b.abs()))
}

/// The kind of a root by the policy's own answer, where it is a priority
/// root.
fn root_kind(action: &PlayerAction) -> Option<u8> {
    match action {
        PlayerAction::PassPriority => Some(0),
        PlayerAction::ActivateManaAbility { .. } => Some(1),
        PlayerAction::PlayLand { .. } => Some(2),
        PlayerAction::ActivateAbility { .. } => Some(3),
        PlayerAction::CastSpell { .. } => Some(4),
        _ => None,
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
struct Leaves {
    #[serde(default)]
    stopped: u64,
}

/// One searched decision as `games.jsonl` keeps it.
#[derive(Clone, Debug, Deserialize)]
struct Event {
    accepted: bool,
    seat: u8,
    /// The record's input that answered it; `None` where the table refused
    /// the answer, which no input of the record is
    /// (`eval4`'s `bind_fallback_events`).
    #[serde(default)]
    record_input_n: Option<u64>,
    /// The answer played.
    action: PlayerAction,
    candidates: Vec<PlayerAction>,
    #[serde(default)]
    scripts: Vec<Option<PlayerAction>>,
    worlds: Vec<Vec<f64>>,
    chosen: usize,
    #[serde(default)]
    leaves: Leaves,
    #[serde(default)]
    diffs: Vec<Option<f64>>,
    #[serde(default)]
    ses: Vec<Option<f64>>,
    /// From runtime 0852c2a4b: in each world, the seat's first answer at
    /// priority after the policy's own when that is a bare mana ability
    /// (`null` otherwise). Other new fields (`ends`, `taken_back`) are read
    /// by nothing here.
    #[serde(default)]
    own_intents: Vec<Option<PlayerAction>>,
}

/// What became of a game's events, by reason.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub(super) struct Counts {
    pub(super) games: u64,
    pub(super) events: u64,
    pub(super) labelled: u64,
    pub(super) dropped: BTreeMap<String, u64>,
    /// Where each root's chain spell (`intents.bin`) came from.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) intents: BTreeMap<String, u64>,
}

impl Counts {
    fn drop(&mut self, reason: &str) {
        *self.dropped.entry(reason.to_owned()).or_default() += 1;
    }

    pub(super) fn add(&mut self, other: &Self) {
        self.games += other.games;
        self.events += other.events;
        self.labelled += other.labelled;
        for (reason, n) in &other.dropped {
            *self.dropped.entry(reason.clone()).or_default() += n;
        }
        for (source, n) in &other.intents {
            *self.intents.entry(source.clone()).or_default() += n;
        }
    }
}

/// A game's roots: the events that may be labelled, by seat and record
/// input, and what the selection already left out.
#[derive(Clone, Debug)]
pub(super) struct GameRoots {
    events: BTreeMap<(u8, u64), Event>,
    counts: Counts,
}

impl GameRoots {
    /// The roots of one `games.jsonl` entry.
    ///
    /// # Errors
    /// An entry without search events, or one that does not parse.
    pub(super) fn from_game(entry: &Value) -> anyhow::Result<Self> {
        let events: Vec<Event> = serde_json::from_value(
            entry
                .get("search_events")
                .cloned()
                .context("a search-advantage run's game has no search_events")?,
        )
        .context("search_events")?;
        let mut counts = Counts {
            games: 1,
            events: events.len() as u64,
            ..Counts::default()
        };
        let clean = entry["fallback_answers"].as_u64() == Some(0)
            && entry["refused_answers"].as_u64() == Some(0);
        let mut kept = BTreeMap::new();
        for event in events {
            let reason = if !clean {
                Some("game_fallback")
            } else if !event.accepted {
                Some("not_accepted")
            } else if event.record_input_n.is_none() {
                Some("unbound")
            } else if event.leaves.stopped != 0 {
                Some("stopped")
            } else if event.candidates.first().and_then(root_kind).is_none() {
                Some("not_priority")
            } else if event.worlds.is_empty()
                || event
                    .worlds
                    .iter()
                    .any(|world| world.len() != event.candidates.len())
                || event.chosen >= event.candidates.len()
            {
                Some("malformed")
            } else if event.worlds.len() < 2 {
                // One world has no standard error, and an item that was a
                // candidate must have one.
                Some("one_world")
            } else {
                None
            };
            match (reason, event.record_input_n) {
                (None, Some(input)) => {
                    ensure!(
                        kept.insert((event.seat, input), event).is_none(),
                        "two search events answer one record input"
                    );
                }
                (reason, _) => counts.drop(reason.unwrap_or("unbound")),
            }
        }
        Ok(Self {
            events: kept,
            counts,
        })
    }

    /// The roots the replay is asked for; with `afterstates`, every root's
    /// candidates too, for their afterstates (`--search-afterstates`).
    pub(super) fn roots(&self, afterstates: bool) -> Roots4 {
        Roots4 {
            roots: self.events.keys().copied().collect::<BTreeSet<_>>(),
            menu: payable_menu,
            afterstates: if afterstates {
                self.events
                    .iter()
                    .map(|(key, event)| {
                        let candidates = event
                            .candidates
                            .iter()
                            .enumerate()
                            .map(|(at, action)| AfterCandidate4 {
                                action: action.clone(),
                                script: event.scripts.get(at).cloned().flatten(),
                            })
                            .collect();
                        (*key, candidates)
                    })
                    .collect()
            } else {
                BTreeMap::new()
            },
        }
    }
}

/// A root's payable menu, as the student's second pass builds it.
fn payable_menu(
    view: &PlayerView,
    pending: &baylee_engine::choice::Pending,
    own: &EncodedCandidates4,
    encode: &dyn Fn(&baylee_engine::choice::Pending) -> EncodedCandidates4,
) -> (EncodedCandidates4, usize) {
    let mut menu = own.clone();
    let mut payable = baylee_train::netplay4::PayableMenu4::new(0.0);
    payable.extend(&mut menu, view, pending, |question| encode(question));
    let first_added = payable.first_added();
    (menu, first_added)
}

/// One labelled root's index row and item rows, before their offset.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Row {
    game: u32,
    cursor: u32,
    seat: u8,
    kind: u8,
    own: u32,
    choice4: u32,
    half_a: u32,
    half_b: u32,
    /// Each half's choice judged on the other half: `half_a`'s (worlds 0 and
    /// 2) mean advantage on worlds 1 and 3, `half_b`'s on worlds 0 and 2
    /// (`search_labels::halves`), 0 where a half kept the policy's answer.
    judged: [f32; 2],
    /// Where the search kept a bare tap: the spell its chain cast at the
    /// table ([`chain_spell`]), `u32::MAX` elsewhere.
    intent: u32,
    items: Vec<Item>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Item {
    advantage: f32,
    se: f32,
    searched: bool,
    added: bool,
    class: u8,
}

/// Why a root is left out, or the labels of it.
fn label(event: &Event, root: &SearchRoot4) -> Result<(Vec<usize>, Vec<Item>), &'static str> {
    let items = macro_items(
        &root.menu,
        &root.pending,
        root.first_added,
        event.candidates.iter().zip(
            event
                .scripts
                .iter()
                .map(Option::as_ref)
                .chain(std::iter::repeat(None)),
        ),
    )
    .map_err(baylee_train::search_labels::MacroItemError4::key)?;
    let mut rows: Vec<Item> = (0..root.menu.candidates.len())
        .map(|at| Item {
            advantage: f32::NAN,
            se: f32::NAN,
            searched: false,
            added: at >= root.first_added,
            class: root.classes.get(at).copied().unwrap_or(u8::MAX),
        })
        .collect();
    for (candidate, &item) in items.iter().enumerate() {
        let (advantage, se) = paired(&event.worlds, candidate);
        if let Some(Some(recorded)) = event.diffs.get(candidate)
            && !same(advantage, *recorded)
        {
            return Err("label_mismatch");
        }
        if let (Some(Some(recorded)), Some(se)) = (event.ses.get(candidate), se)
            && !same(se, *recorded)
        {
            return Err("label_mismatch");
        }
        #[allow(clippy::cast_possible_truncation, reason = "labels are f32")]
        let row = Item {
            advantage: advantage as f32,
            se: se.map_or(f32::NAN, |se| se as f32),
            searched: true,
            ..rows[item]
        };
        rows[item] = row;
    }
    Ok((items, rows))
}

/// A game being converted: its roots, its twins' stream and its rows.
pub(super) struct GameLabels<'a> {
    roots: &'a GameRoots,
    /// The record's inputs: seat, number and answer, in order.
    inputs: Vec<(u8, u64, PlayerAction)>,
    twins: super::output::GameWriter,
    rows: Vec<Row>,
    counts: Counts,
    /// Roots the replay met, labelled or dropped.
    met: usize,
    /// `--search-afterstates`: the game's afterstates.
    afterstates: Option<super::afterstates::GameAfterstates>,
}

/// A game's afterstate rows, counts and frame shard.
pub(super) type GameAfter = (
    Vec<super::afterstates::AfterRow>,
    super::afterstates::AfterCounts,
    String,
);

/// A finished game's label rows, counts, twin shard and afterstates.
pub(super) type GameFinished = (Vec<Row>, Counts, Option<String>, Option<GameAfter>);

impl<'a> GameLabels<'a> {
    /// Starts a game's labels; its twins go to the dataset `root`'s menus
    /// split, and with `afterstates` its afterstates to their own.
    ///
    /// # Errors
    /// When a stream cannot be opened.
    pub(super) fn new(
        roots: &'a GameRoots,
        root: &Path,
        game_id: u32,
        rule_roots: std::sync::Arc<BTreeMap<String, u32>>,
        record: &[u8],
        afterstates: bool,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            roots,
            inputs: record_inputs(record)?,
            twins: super::output::GameWriter::in_split(
                root,
                MENUS_SPLIT,
                game_id,
                rule_roots.clone(),
            )?,
            rows: Vec::new(),
            counts: roots.counts.clone(),
            met: 0,
            afterstates: afterstates
                .then(|| super::afterstates::GameAfterstates::new(root, game_id, rule_roots))
                .transpose()?,
        })
    }

    /// Labels a frame where it is a root, writes its twin, and leaves every
    /// frame at policy weight 0.
    ///
    /// # Errors
    /// A root whose played answer is not the record's, or a twin that
    /// cannot be written.
    pub(super) fn apply(&mut self, step: &mut Step4) -> Result<(), String> {
        step.loss_weight = 0.0;
        let Some(root) = step.search_root.take() else {
            return Ok(());
        };
        let roots = self.roots;
        let Some(event) = roots.events.get(&(step.seat, root.record_input_n)) else {
            return Err(format!(
                "a root the replay kept at input {} has no search event",
                root.record_input_n
            ));
        };
        self.met += 1;
        // The event binds to its frame: what it played is the record's answer.
        let played = macro_item(
            &root.menu,
            &root.pending,
            root.first_added,
            &event.action,
            None,
        )
        .ok()
        .and_then(|item| u16::try_from(item).ok());
        if played != Some(step.target.single_idx) {
            return Err(format!(
                "search event at input {} played {:?}, which is not the record's answer",
                root.record_input_n, event.action
            ));
        }
        let (items, rows) = match label(event, &root) {
            Ok(labels) => labels,
            Err(reason) => {
                self.counts.drop(reason);
                return Ok(());
            }
        };
        let halves = halves(&event.worlds, HALVES_MARGIN);
        let index = |candidate: usize| {
            items
                .get(candidate)
                .and_then(|&item| u32::try_from(item).ok())
                .unwrap_or(u32::MAX)
        };
        let twin = twin_of(step, &root);
        self.twins
            .push_with_block(&twin, &twin_block(step.step_cursor))
            .map_err(|e| e.to_string())?;
        if let Some(after) = &mut self.afterstates {
            let Some(captured) = &root.afterstates else {
                return Err(format!(
                    "the replay captured no afterstates at input {}",
                    root.record_input_n
                ));
            };
            after.record(
                step,
                captured,
                &super::afterstates::EventCandidates {
                    candidates: &event.candidates,
                    scripts: &event.scripts,
                    worlds: &event.worlds,
                    items: &items,
                },
            )?;
        }
        let intent = self.intent(event, step.seat, root.record_input_n);
        #[allow(clippy::cast_possible_truncation, reason = "labels are f32")]
        let judged = [halves.judged_even as f32, halves.judged_odd as f32];
        self.counts.labelled += 1;
        self.rows.push(Row {
            game: step.game_id,
            cursor: step.step_cursor,
            seat: step.seat,
            kind: event
                .candidates
                .first()
                .and_then(root_kind)
                .unwrap_or(u8::MAX),
            own: index(0),
            choice4: index(event.chosen),
            half_a: index(halves.on_even),
            half_b: index(halves.on_odd),
            judged,
            // Kept: the record follows the bundle's own chain from its tap.
            intent,
            items: rows,
        });
        Ok(())
    }

    /// A root's chain spell (`intents.bin`): where the event says what the
    /// policy's bare tap was for (`own_intents`, runtime 0852c2a4b on), the
    /// spell every world names; else, where the search kept the tap, the
    /// spell the record's chain cast; `u32::MAX` elsewhere. Counted by
    /// where it came from, and where the events and the record both say,
    /// whether they differ.
    fn intent(&mut self, event: &Event, seat: u8, n: u64) -> u32 {
        if kind_of(event) != Some(1) {
            return u32::MAX;
        }
        let from_record = (event.chosen == 0).then(|| chain_spell(&self.inputs, seat, n));
        let (intent, source) = intent_of(event, from_record);
        let mut count = |key: &str| *self.counts.intents.entry(key.to_owned()).or_default() += 1;
        count(source);
        if source == "events" && from_record.is_some_and(|spell| spell != intent) {
            count("record_differs");
        }
        intent
    }

    /// Publishes the game's twins and gives its rows, counts and twin shard.
    /// A root the replay gave no frame for is counted as `no_frame`, so that
    /// every event is labelled or dropped for a reason.
    ///
    /// # Errors
    /// When the twins' stream cannot be published.
    pub(super) fn finish(mut self) -> anyhow::Result<GameFinished> {
        for _ in self.met..self.roots.events.len() {
            self.counts.drop("no_frame");
        }
        let shard = self.twins.finish()?;
        let after = self
            .afterstates
            .map(super::afterstates::GameAfterstates::finish)
            .transpose()?;
        Ok((self.rows, self.counts, Some(shard), after))
    }
}

/// A tap root's chain spell and where it came from: the spell every world
/// of the event names (`own_intents`), else the record's (`from_record`,
/// where the search kept the tap).
fn intent_of(event: &Event, from_record: Option<u32>) -> (u32, &'static str) {
    if event.own_intents.is_empty() {
        return from_record.map_or((u32::MAX, "none"), |spell| (spell, "record"));
    }
    let named: Vec<u32> = event
        .own_intents
        .iter()
        .flatten()
        .map(|action| spell_card(action).unwrap_or(u32::MAX))
        .collect();
    match named.split_first() {
        Some((first, rest)) if rest.iter().all(|other| other == first) => (*first, "events"),
        Some(_) => (u32::MAX, "events_disagree"),
        None => (u32::MAX, "events_none"),
    }
}

/// The kind of an event's root ([`ROOT_KINDS`]).
fn kind_of(event: &Event) -> Option<u8> {
    event.candidates.first().and_then(root_kind)
}

/// The seat, number and answer of every input of a record, in order.
///
/// # Errors
/// A line that is no record line.
fn record_inputs(record: &[u8]) -> anyhow::Result<Vec<(u8, u64, PlayerAction)>> {
    let mut inputs = Vec::new();
    for line in record
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
    {
        if let baylee_gamehost::record::Line::Input {
            seat, n, action, ..
        } = serde_json::from_slice(line).context("a record line")?
        {
            inputs.push((seat, n, action));
        }
    }
    Ok(inputs)
}

/// The most of a seat's inputs a chain is followed for.
const CHAIN: usize = 16;

/// The spell a chain begun at `seat`'s input `n` cast at the table: the
/// card of the seat's first cast, suspend or foretell after it, as the
/// record names it, before the seat passes, cancels or answers anything
/// but a mana ability; `u32::MAX` where it does none of those within
/// [`CHAIN`] of its inputs. An activated ability may be a mana ability,
/// which the record does not say, so it does not end a chain.
fn chain_spell(inputs: &[(u8, u64, PlayerAction)], seat: u8, n: u64) -> u32 {
    inputs
        .iter()
        .skip_while(|(_, at, _)| *at <= n)
        .filter(|(by, _, _)| *by == seat)
        .take(CHAIN)
        .find_map(|(_, _, action)| match action {
            PlayerAction::ActivateManaAbility { .. }
            | PlayerAction::ChooseManaAbility { .. }
            | PlayerAction::ActivateAbility { .. } => None,
            other => Some(spell_card(other).unwrap_or(u32::MAX)),
        })
        .unwrap_or(u32::MAX)
}

/// The card a cast, a suspend or a foretell names, as the record and the
/// events write it; `None` for any other answer.
fn spell_card(action: &PlayerAction) -> Option<u32> {
    match action {
        PlayerAction::CastSpell { card }
        | PlayerAction::Suspend { card }
        | PlayerAction::Foretell { card } => Some(
            serde_json::to_value(card)
                .ok()
                .and_then(|value| value.as_u64())
                .and_then(|id| u32::try_from(id).ok())
                .unwrap_or(u32::MAX),
        ),
        _ => None,
    }
}

/// A root's twin: its frame with the payable menu, at policy weight 0; the
/// recorded answer keeps its index (the payable menu adds after the frame's
/// own items).
fn twin_of(step: &Step4, root: &SearchRoot4) -> Step4 {
    let menu = &root.menu;
    let mut twin = step.clone();
    twin.loss_weight = 0.0;
    twin.cand_classes = None;
    twin.distill = None;
    twin.search_root = None;
    twin.cand_rule_refs = menu
        .candidates
        .iter()
        .map(|c| baylee_train::cardstructure::refs::candidate_refs(c, &step.state))
        .collect();
    twin.cand_feat = menu.candidates.iter().map(|c| c.feat.clone()).collect();
    twin.cand_entity_idx = menu.candidates.iter().map(|c| c.entity_idx).collect();
    twin.cand_rel_seat_idx = menu.candidates.iter().map(|c| c.rel_seat_idx).collect();
    twin.cand_card_id = menu.candidates.iter().map(|c| c.card_id).collect();
    twin.cand_face_id = menu
        .candidates
        .iter()
        .map(baylee_train::features4::Candidate4::named_face)
        .collect();
    twin.cand_ability_idx = menu.candidates.iter().map(|c| c.ability_idx).collect();
    twin
}

/// Writes the sidecar under `dir` and returns its header.
///
/// # Errors
/// When a file exists already or cannot be written.
pub(super) fn write(
    dir: &Path,
    mut rows: Vec<Row>,
    counts: &Counts,
    extra: &Value,
) -> anyhow::Result<Value> {
    fs::create_dir_all(dir)?;
    let paths = [
        "index.bin",
        "cands.bin",
        "header.json",
        "intents.bin",
        "halves-judged.bin",
    ]
    .map(|name| dir.join(name));
    for path in &paths {
        ensure!(!path.exists(), "{} exists", path.display());
    }
    rows.sort_by_key(|row| (row.game, row.seat, row.cursor));
    let mut index = Vec::with_capacity(rows.len() * INDEX_ROW);
    let mut cands = Vec::new();
    let mut intents = Vec::with_capacity(rows.len() * 4);
    let mut judged = Vec::with_capacity(rows.len() * 8);
    for row in &rows {
        intents.extend_from_slice(&row.intent.to_le_bytes());
        for value in row.judged {
            judged.extend_from_slice(&value.to_le_bytes());
        }
        let offset = (cands.len() / CAND_ROW) as u64;
        index.extend_from_slice(&row.game.to_le_bytes());
        index.extend_from_slice(&row.cursor.to_le_bytes());
        index.extend_from_slice(&row.cursor.to_le_bytes());
        index.extend_from_slice(&[row.seat, row.kind, 0, 0]);
        index.extend_from_slice(&row.own.to_le_bytes());
        index.extend_from_slice(&u32::try_from(row.items.len())?.to_le_bytes());
        for at in [row.choice4, row.half_a, row.half_b] {
            index.extend_from_slice(&at.to_le_bytes());
        }
        index.extend_from_slice(&offset.to_le_bytes());
        index.extend_from_slice(&[0; 4]);
        for item in &row.items {
            cands.extend_from_slice(&item.advantage.to_le_bytes());
            cands.extend_from_slice(&item.se.to_le_bytes());
            cands.extend_from_slice(&[
                u8::from(item.searched),
                u8::from(item.added),
                item.class,
                0,
            ]);
        }
    }
    fs::write(&paths[0], &index)?;
    fs::write(&paths[1], &cands)?;
    fs::write(&paths[3], &intents)?;
    fs::write(&paths[4], &judged)?;
    let header = json!({
        "contract": SEARCH_ADVANTAGE_CONTRACT,
        "version": 1,
        "key": ["game", "seat", "cursor"],
        "twins": {"split": MENUS_SPLIT, "meta_key": MENUS_SHARDS_KEY,
            "block": "16 bytes after the payload: b\"BSA1\", kind 1, three zero bytes, \
                      the root cursor u32 LE, four zero bytes"},
        "index": {"file": "index.bin", "row_bytes": INDEX_ROW, "rows": rows.len(),
            "sorted_by": ["game", "seat", "cursor"],
            "dtype": [["game", "<u4"], ["cursor", "<u4"], ["root_cursor", "<u4"],
                ["seat", "u1"], ["root_kind", "u1"], ["pad0", "V2"], ["own_idx", "<u4"],
                ["n_cand", "<u4"], ["choice4_idx", "<u4"], ["half_a_idx", "<u4"],
                ["half_b_idx", "<u4"], ["offset", "<u8"], ["pad1", "V4"]],
            "root_kinds": ROOT_KINDS,
            "halves": {"half_a": "worlds 0 and 2", "half_b": "worlds 1 and 3",
                "margin": HALVES_MARGIN, "rule": "baylee_train::search_labels::halves"},
            "none": u32::MAX,
            "sha256": super::shadow::sha(&index)},
        "cands": {"file": "cands.bin", "row_bytes": CAND_ROW, "rows": cands.len() / CAND_ROW,
            "dtype": [["advantage", "<f4"], ["se", "<f4"], ["searched", "u1"],
                ["payable_added", "u1"], ["coarse_class", "u1"], ["pad", "V1"]],
            "advantage": "mean over the worlds of the item's score less the policy's own \
                          (search_labels::paired), NaN where the item was no candidate; \
                          the policy's own item 0 and 0",
            "coarse_classes": baylee_train::coarse_class::COARSE_CLASSES,
            "sha256": super::shadow::sha(&cands)},
        "intents": {"file": "intents.bin", "dtype": "<u4", "rows": rows.len(),
            "order": "the index's",
            "meaning": "at a tap root: where the event says what the policy's bare tap \
                        was for (own_intents, runtime 0852c2a4b on), the card every world \
                        names; else, where the search kept the tap (choice4 the own \
                        answer), the card the seat's chain then cast, suspended or \
                        foretold at the table, as the record names it; u32::MAX elsewhere, \
                        where the worlds disagree or the chain passed first. Counted by \
                        source in counts.intents. For analysis only.",
            "sha256": super::shadow::sha(&intents)},
        "halves_judged": {"file": "halves-judged.bin", "rows": rows.len(),
            "order": "the index's", "dtype": [["a_on_b", "<f4"], ["b_on_a", "<f4"]],
            "meaning": "half_a's choice (worlds 0 and 2) judged on worlds 1 and 3, and \
                        half_b's (1 and 3) on 0 and 2: its mean advantage there over the \
                        policy's own answer, 0 where the half kept it \
                        (search_labels::halves, margin 0.03)",
            "sha256": super::shadow::sha(&judged)},
        "counts": counts,
        "source": extra,
    });
    fs::write(&paths[2], serde_json::to_vec_pretty(&header)?)
        .with_context(|| format!("writing {}", paths[2].display()))?;
    Ok(header)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod chain_tests;

#[cfg(test)]
mod intent_tests;
