//! Search distillation (`--search-targets`): the answers a lookahead search
//! played become the policy targets, so the net learns what its search
//! found. Only the net seat's searched decisions keep their weight (its
//! unsearched ones at `--search-unsearched-weight`, and searched decisions
//! that kept the policy's own move at `--search-unchanged-weight`); a house
//! opponent's, forced answers and every other step keep their frame for
//! memory and value but no policy weight.

use anyhow::{Context as _, ensure};
use baylee_train::convert4::Step4;
use baylee_train::features4::{MAX_SEATS, PendingKind4, SourceKind4};
use serde_json::{Value, json};

const CONTRACT: &str = "search_distillation_v1";

/// The runs a search-distillation dataset is made of, and its weighting.
pub(super) struct Targets {
    weighting: Weighting,
    searches: Vec<Value>,
}

/// The policy weight and target mode each step keeps; copied into every
/// conversion worker.
#[derive(Clone, Copy, Debug)]
pub(super) struct Weighting {
    unsearched_net_weight: f32,
    unchanged_search_weight: f32,
    turn_value_target: bool,
}

/// One accepted searched priority decision recorded in `games.jsonl`.
#[derive(Clone, Copy, Debug)]
struct SearchedEvent {
    turn: u16,
    changed: bool,
    rejected: bool,
    root_value: Option<f32>,
    diff: f32,
}

fn action_kind(action: &Value) -> &str {
    match action {
        Value::String(kind) => kind.as_str(),
        Value::Object(fields) => fields.keys().next().map_or("", String::as_str),
        _ => "",
    }
}

fn is_rejected_override(own_kind: &str, played_kind: &str) -> bool {
    own_kind == "ActivateManaAbility"
        || played_kind == "ActivateManaAbility"
        || played_kind == "CancelCast"
        || (matches!(own_kind, "PlayLand" | "CastSpell" | "ActivateAbility")
            && played_kind == "PassPriority")
}

/// Per-seat accepted search events for a single recorded game.
#[derive(Clone, Debug, Default)]
pub(super) struct GameSearchEvents {
    seats: [Vec<SearchedEvent>; MAX_SEATS],
}

/// Stateful per-game cursor matching emitted [`Step4`] frames to [`GameSearchEvents`].
pub(super) struct GameCursor<'a> {
    events: Option<&'a GameSearchEvents>,
    weighting: Weighting,
    next_event: [usize; MAX_SEATS],
    last_changed: [bool; MAX_SEATS],
    last_rejected: [bool; MAX_SEATS],
    last_diff: [f32; MAX_SEATS],
    last_root_value: [Option<f32>; MAX_SEATS],
}

impl Targets {
    /// Checks the weights of the net's unsearched and unchanged-search decisions.
    #[cfg(test)]
    pub(super) fn new(unsearched_net_weight: f32) -> anyhow::Result<Self> {
        Self::with_options(unsearched_net_weight, 1.0, false)
    }

    /// Checks all search-distillation weighting options.
    pub(super) fn with_options(
        unsearched_net_weight: f32,
        unchanged_search_weight: f32,
        turn_value_target: bool,
    ) -> anyhow::Result<Self> {
        ensure!(
            unsearched_net_weight.is_finite() && (0.0..=1.0).contains(&unsearched_net_weight),
            "--search-unsearched-weight {unsearched_net_weight} outside [0, 1]"
        );
        ensure!(
            unchanged_search_weight.is_finite() && (0.0..=1.0).contains(&unchanged_search_weight),
            "--search-unchanged-weight {unchanged_search_weight} outside [0, 1]"
        );
        Ok(Self {
            weighting: Weighting {
                unsearched_net_weight,
                unchanged_search_weight,
                turn_value_target,
            },
            searches: Vec::new(),
        })
    }

    /// The weighting the conversion workers apply.
    pub(super) fn weighting(&self) -> Weighting {
        self.weighting
    }

    /// Admits one run: it must have been played with a search.
    pub(super) fn admit(&mut self, run_json: &Value) -> anyhow::Result<()> {
        let search = run_json
            .get("search")
            .filter(|search| !search.is_null())
            .context("--search-targets converts searched runs only; this run has no search")?;
        ensure!(
            search.get("contract").and_then(Value::as_str).is_some(),
            "the run's search names no contract"
        );
        self.searches.push(search.clone());
        Ok(())
    }

    /// The dataset metadata naming these targets.
    pub(super) fn metadata(&self, games: usize) -> Value {
        let mut meta = json!({"version": 1, "contract": CONTRACT, "games": games,
            "targets": "the answers searched decisions played",
            "unsearched_net_weight": self.weighting.unsearched_net_weight,
            "searches": self.searches});
        if (self.weighting.unchanged_search_weight - 1.0).abs() > f32::EPSILON {
            meta["unchanged_search_weight"] = json!(self.weighting.unchanged_search_weight);
        }
        if self.weighting.turn_value_target {
            meta["turn_value_target"] = json!(true);
        }
        meta
    }
}

impl GameSearchEvents {
    /// Parses `search_events` from a `games.jsonl` entry.
    pub(super) fn from_game(game: &Value) -> Self {
        let mut seats: [Vec<SearchedEvent>; MAX_SEATS] = std::array::from_fn(|_| Vec::new());
        let Some(events) = game.get("search_events").and_then(Value::as_array) else {
            return Self { seats };
        };
        for event in events {
            if event.get("accepted").and_then(Value::as_bool) != Some(true) {
                continue;
            }
            let Some(seat) = event
                .get("seat")
                .and_then(Value::as_u64)
                .and_then(|s| usize::try_from(s).ok())
                .filter(|&s| s < MAX_SEATS)
            else {
                continue;
            };
            let chosen = event
                .get("chosen")
                .and_then(Value::as_u64)
                .and_then(|c| usize::try_from(c).ok())
                .unwrap_or(0);
            let turn = event
                .get("turn")
                .and_then(Value::as_u64)
                .map_or(0, |t| t.min(u64::from(u16::MAX)) as u16);
            #[allow(clippy::cast_possible_truncation)]
            let root_value = event
                .get("root_value")
                .and_then(Value::as_f64)
                .or_else(|| {
                    event
                        .get("means")
                        .and_then(Value::as_array)
                        .and_then(|m| m.first())
                        .and_then(Value::as_f64)
                })
                .map(|v| v as f32)
                .filter(|v| v.is_finite());
            #[allow(clippy::cast_possible_truncation)]
            let diff = event
                .get("diffs")
                .and_then(Value::as_array)
                .and_then(|d| d.get(chosen))
                .and_then(Value::as_f64)
                .map_or(0.0, |v| v as f32);
            let changed = chosen != 0;
            let cand_len = event
                .get("candidates")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            let own_kind = event
                .get("candidates")
                .and_then(Value::as_array)
                .and_then(|c| c.first())
                .map_or("", action_kind);
            let played_kind = event.get("action").map_or("", action_kind);
            let rejected = (cand_len <= 1 && own_kind == "CastSpell")
                || (changed && is_rejected_override(own_kind, played_kind));
            seats[seat].push(SearchedEvent {
                turn,
                changed,
                rejected,
                root_value,
                diff: if diff.is_finite() { diff } else { 0.0 },
            });
        }
        Self { seats }
    }

    fn next_turn_value(&self, seat: usize, turn: u16, after_idx: usize, terminal: f32) -> f32 {
        self.seats[seat]
            .get(after_idx..)
            .into_iter()
            .flatten()
            .find(|ev| ev.turn > turn && ev.root_value.is_some())
            .and_then(|ev| ev.root_value)
            .unwrap_or(terminal)
    }

    fn turn_baseline(&self, seat: usize, turn: u16) -> Option<(usize, f32)> {
        let events = &self.seats[seat];
        if let Some((idx, ev)) = events
            .iter()
            .enumerate()
            .find(|(_, ev)| ev.turn >= turn && ev.root_value.is_some())
        {
            return ev.root_value.map(|v| (idx, v));
        }
        events
            .iter()
            .enumerate()
            .rev()
            .find(|(_, ev)| ev.root_value.is_some())
            .and_then(|(idx, ev)| ev.root_value.map(|v| (idx, v)))
    }
}

impl Weighting {
    /// Creates a per-game cursor to weight and relabel steps in stream order.
    pub(super) fn cursor(self, events: Option<&GameSearchEvents>) -> GameCursor<'_> {
        GameCursor {
            events,
            weighting: self,
            next_event: [0; MAX_SEATS],
            last_changed: [true; MAX_SEATS],
            last_rejected: [false; MAX_SEATS],
            last_diff: [0.0; MAX_SEATS],
            last_root_value: [None; MAX_SEATS],
        }
    }

    fn weight(self, is_decision: bool, source: SourceKind4, changed: bool, rejected: bool) -> f32 {
        match (is_decision, source) {
            (true, SourceKind4::Search) => {
                if rejected {
                    0.0
                } else if changed {
                    1.0
                } else {
                    self.unchanged_search_weight
                }
            }
            (true, SourceKind4::Net) => self.unsearched_net_weight,
            _ => 0.0,
        }
    }
}

impl GameCursor<'_> {
    /// Applies search-distillation step weighting and optional dense next-turn
    /// value targets to `step`.
    pub(super) fn apply(&mut self, step: &mut Step4) {
        let seat = usize::from(step.seat).min(MAX_SEATS - 1);
        let mut changed = true;
        let mut rejected = false;
        let mut search_diff = 0.0_f32;
        let mut base_value = None;
        let mut event_idx = self.next_event[seat];

        if step.is_decision && step.provenance.source == SourceKind4::Search {
            if let Some(events) = self.events {
                if step.pending_kind == PendingKind4::Priority as u8 {
                    if let Some(ev) = events.seats[seat].get(self.next_event[seat]) {
                        event_idx = self.next_event[seat];
                        self.next_event[seat] += 1;
                        changed = ev.changed;
                        rejected = ev.rejected;
                        search_diff = ev.diff;
                        base_value = ev.root_value;
                        self.last_changed[seat] = changed;
                        self.last_rejected[seat] = rejected;
                        self.last_diff[seat] = search_diff;
                        self.last_root_value[seat] = base_value;
                    }
                } else {
                    // A target question answered by `follow_decision` inherits
                    // its cast's search decision.
                    event_idx = self.next_event[seat].saturating_sub(1);
                    changed = self.last_changed[seat];
                    rejected = self.last_rejected[seat];
                    search_diff = self.last_diff[seat];
                    base_value = self.last_root_value[seat];
                }
            }
        } else if step.is_decision
            && step.provenance.source == SourceKind4::Net
            && let Some(events) = self.events
            && let Some((idx, v)) = events.turn_baseline(seat, step.turn)
        {
            event_idx = idx;
            base_value = Some(v);
        }

        step.loss_weight *=
            self.weighting
                .weight(step.is_decision, step.provenance.source, changed, rejected);

        if self.weighting.turn_value_target
            && let Some(events) = self.events
            && let Some(v_base) =
                base_value.or_else(|| events.turn_baseline(seat, step.turn).map(|(_, v)| v))
        {
            let v_next = events.next_turn_value(seat, step.turn, event_idx + 1, step.outcome);
            let delta_v = v_next - v_base;
            let advantage =
                if step.is_decision && step.provenance.source == SourceKind4::Search && changed {
                    delta_v.max(search_diff).max(0.0)
                } else {
                    delta_v
                };
            step.outcome = (0.5 + 2.0 * advantage).clamp(0.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_searched_decisions_keep_full_weight() {
        let weighting = Targets::with_options(0.25, 0.5, false)
            .expect("weight in range")
            .weighting();
        let weight = |source, changed| weighting.weight(true, source, changed, false);
        assert!((weight(SourceKind4::Search, true) - 1.0).abs() < f32::EPSILON);
        assert!((weight(SourceKind4::Search, false) - 0.5).abs() < f32::EPSILON);
        assert!(
            weighting
                .weight(true, SourceKind4::Search, true, true)
                .abs()
                < f32::EPSILON
        );
        assert!((weight(SourceKind4::Net, false) - 0.25).abs() < f32::EPSILON);
        for source in [
            SourceKind4::House,
            SourceKind4::Forced,
            SourceKind4::External,
        ] {
            assert!(
                weight(source, true).abs() < f32::EPSILON,
                "{source:?} is no target"
            );
        }
    }

    #[test]
    fn an_observation_frame_is_no_target() {
        let weighting = Targets::new(1.0).expect("weight in range").weighting();
        for source in [SourceKind4::Search, SourceKind4::Net] {
            assert!(weighting.weight(false, source, true, false).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn a_run_without_search_is_refused() {
        let mut targets = Targets::new(0.0).expect("weight in range");
        assert!(targets.admit(&json!({"search": null})).is_err());
        assert!(targets.admit(&json!({})).is_err());
        assert!(targets.admit(&json!({"search": {}})).is_err());
        targets
            .admit(&json!({"search": {"contract": "belief_lookahead_v2"}}))
            .expect("a searched run");
        assert_eq!(
            targets.metadata(3)["searches"].as_array().map(Vec::len),
            Some(1)
        );
    }

    #[test]
    fn weights_outside_the_unit_interval_are_refused() {
        for weight in [-0.1, 1.5, f32::NAN] {
            assert!(Targets::new(weight).is_err(), "{weight}");
            assert!(
                Targets::with_options(0.25, weight, false).is_err(),
                "{weight}"
            );
        }
    }
}
