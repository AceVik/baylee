//! A game's record, turned into the decisions a net learns from: encoder v3
//! ([`crate::features3`]).
//!
//! The record is replayed and hash checked as [`crate::convert`] does for v2,
//! which stays as it is for the nets trained on it. What v3 adds is what a
//! table of more than two seats needs: every seat's side and deck list, from
//! the record's preset, and per decision who won and who was still in,
//! relative to the deciding seat.

use std::collections::BTreeMap;

use baylee_core::ids::{CardIndex, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_gamehost::RegistryLookup;
use baylee_gamehost::record::{Line, RECORD_VERSION};
use baylee_view::PlayerView;

pub use crate::convert::{Keep, Refused};
use crate::features::{rel, seat_view};
use crate::features3::{Encoded, Omni, Table, deck_list, encode, omniscient, options};
use crate::policy::{self, Opt, Picked, Unmatched, Unscored};

/// One step of a decision (see [`crate::convert::Sample`]).
#[derive(Clone, Debug)]
pub struct Sample {
    /// The record line it was answered on.
    pub n: u64,
    /// The seat that answered.
    pub seat: u8,
    /// The turn it was asked on.
    pub turn: u32,
    /// Which pick of the answer this is; 0 for the first.
    pub step: u16,
    /// What the seat saw.
    pub actor: Encoded,
    /// The options it had at this step, one per row an answer can name.
    pub opts: Vec<Opt>,
    /// Which of them the house picked, or -1 when its answer could not be
    /// read as one of them.
    pub chosen: i32,
    /// What it did not see (training only).
    pub omni: Omni,
}

/// A converted game.
#[derive(Clone, Debug)]
pub struct Converted {
    /// Its decisions, in order.
    pub samples: Vec<Sample>,
    /// The winning seats; empty for a draw.
    pub winners: Vec<u8>,
    /// The seats still in the game when it ended.
    pub alive: Vec<u8>,
    /// Per seat, its side (`None`: alone).
    pub teams: Vec<Option<u8>>,
    /// The turn it ended on.
    pub final_turn: u32,
    /// Inputs replayed.
    pub inputs: u64,
    /// Answers not read as options, by question kind and why.
    pub unmatched: BTreeMap<(i16, &'static str), u64>,
    /// Where the first few of them are: the input's number, the question's
    /// kind and why, so a report can name the game and the move.
    pub unmatched_at: Vec<(u64, i16, &'static str)>,
}

impl Converted {
    /// The seats `seats` as a bit mask relative to `seat`: bit `r` is the
    /// seat `r` places after it in turn order.
    #[must_use]
    pub fn relative_mask(&self, seat: u8, seats: &[u8]) -> i64 {
        let n = self.teams.len();
        seats
            .iter()
            .map(|s| rel(PlayerId::new(seat), PlayerId::new(*s), n))
            .fold(0, |m, r| m | (1 << r))
    }

    /// `seat`'s side, relative to it: every seat on its team, itself too.
    #[must_use]
    pub fn team_mask(&self, seat: u8) -> i64 {
        let mine = self.teams.get(usize::from(seat)).copied().flatten();
        let side: Vec<u8> = (0..self.teams.len())
            .map(|s| s as u8)
            .filter(|s| *s == seat || (mine.is_some() && self.teams[usize::from(*s)] == mine))
            .collect();
        self.relative_mask(seat, &side)
    }
}

/// One step of a decision before it becomes a [`Sample`].
type StepSample = (u16, Encoded, Vec<Opt>, i32);

/// The steps of one decision, and why its answer could not be read, if not.
fn decision(
    view: &PlayerView,
    pending: &Pending,
    action: &PlayerAction,
    table: &Table<'_>,
    base: Encoded,
) -> (Vec<StepSample>, Option<&'static str>) {
    let hand: Vec<_> = view.hand.iter().map(|h| h.id).collect();
    let seats = view.seats.len();
    match policy::steps(pending, &hand, action) {
        Ok(steps) => {
            let mut out = Vec::with_capacity(steps.len());
            let mut why = None;
            let mut base = Some(base);
            for (k, step) in steps.iter().enumerate() {
                let actor = match base.take() {
                    Some(b) if k == 0 => b,
                    _ => encode(view, pending, &step.picked, table),
                };
                let pairs = options(&actor, view, pending, &step.picked);
                // A pile's member the house took is the pile's row: the
                // option with the same triple.
                let row_of = |id| actor.slots.get(&id).copied();
                let rel_of = |p| rel(view.seat, p, seats);
                let taken = policy::opt(step.chosen, &row_of, &rel_of);
                let chosen = taken.and_then(|t| {
                    pairs
                        .iter()
                        .position(|(_, o)| (o.head, o.a, o.b) == (t.head, t.a, t.b))
                });
                if chosen.is_none() {
                    why = Some("no_row");
                }
                let opts = pairs.into_iter().map(|(_, o)| o).collect();
                let chosen = chosen.map_or(-1, |i| i32::try_from(i).unwrap_or(-1));
                out.push((u16::try_from(k).unwrap_or(u16::MAX), actor, opts, chosen));
            }
            (out, why)
        }
        Err(e) => {
            let opts = options(&base, view, pending, &Picked::default())
                .into_iter()
                .map(|(_, o)| o)
                .collect();
            let why = match e {
                Unmatched::Unscored(Unscored::Unsupported) => "unsupported",
                Unmatched::Unscored(Unscored::Over) => "over",
                Unmatched::Shape => "shape",
            };
            (vec![(0, base, opts, -1)], Some(why))
        }
    }
}

/// Replays `record` and encodes the decisions `keep` keeps.
///
/// # Errors
/// See [`Refused`]; a refused record contributes nothing.
#[allow(clippy::too_many_lines)] // one replay loop, as v2's
pub fn convert(record: &[u8], keep: Keep) -> Result<Converted, Refused> {
    let mut lines = record
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_slice::<Line>(l).map_err(|e| Refused::Unreadable(e.to_string())));
    let Some(Line::Header {
        record: RECORD_VERSION,
        preset,
        hash,
        ..
    }) = lines.next().transpose()?
    else {
        return Err(Refused::Unreadable("no header this build reads".into()));
    };
    let teams: Vec<Option<u8>> = preset.seats.iter().map(|s| s.team).collect();
    let decks: Vec<Vec<(CardIndex, u32)>> = preset.seats.iter().map(deck_list).collect();
    let mut engine = Engine::new(&preset, RegistryLookup).map_err(|_| Refused::Unbuildable)?;
    let hex = |engine: &Engine<RegistryLookup>| format!("{:016x}", engine.snapshot_hash());
    if hex(&engine) != hash {
        return Err(Refused::Diverged(None));
    }
    let every = keep.every.max(1);
    let mut eligible = 0_u32;
    let mut samples = Vec::new();
    let mut inputs = 0;
    let mut winners = None;
    let mut unmatched: BTreeMap<(i16, &'static str), u64> = BTreeMap::new();
    let mut unmatched_at = Vec::new();
    for line in lines {
        match line? {
            Line::Header { .. } => return Err(Refused::Unreadable("a second header".into())),
            Line::Chair { .. } | Line::DeclaredMind { .. } => {}
            Line::End { winners: w, .. } => winners = Some(w),
            Line::Input {
                n,
                seat,
                action,
                hash,
                ..
            } => {
                let player = PlayerId::new(seat);
                if !action.is_automation_setting()
                    && let Some(pending) = engine.pending_for(player).cloned()
                {
                    let view = seat_view(&engine, player, &pending, n);
                    let deck = decks.get(usize::from(seat)).map_or(&[][..], Vec::as_slice);
                    let table = Table {
                        teams: &teams,
                        deck,
                    };
                    if let Some(base) = encode_decision(&view, &pending, &table)
                        && (keep.forced || base.options > 1)
                    {
                        if eligible.is_multiple_of(every) {
                            let kind = base.kind;
                            let (steps, why) = decision(&view, &pending, &action, &table, base);
                            if let Some(why) = why {
                                *unmatched.entry((kind, why)).or_default() += 1;
                                if unmatched_at.len() < 4 {
                                    unmatched_at.push((n, kind, why));
                                }
                            }
                            let omni = omniscient(&engine, player, &decks);
                            let turn = engine.state().turn.number;
                            for (step, actor, opts, chosen) in steps {
                                samples.push(Sample {
                                    n,
                                    seat,
                                    turn,
                                    step,
                                    actor,
                                    opts,
                                    chosen,
                                    omni: omni.clone(),
                                });
                            }
                        }
                        eligible += 1;
                    }
                }
                engine
                    .apply(player, action)
                    .map_err(|_| Refused::Refused(n))?;
                if hex(&engine) != hash {
                    return Err(Refused::Diverged(Some(n)));
                }
                inputs += 1;
            }
        }
    }
    let winners = winners.ok_or(Refused::Unfinished)?;
    let alive = engine
        .state()
        .players
        .iter()
        .enumerate()
        .filter(|(_, p)| p.loss.is_none())
        .map(|(i, _)| i as u8)
        .collect();
    Ok(Converted {
        samples,
        winners,
        alive,
        teams,
        final_turn: engine.state().turn.number,
        inputs,
        unmatched,
        unmatched_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features3::ENT_COLS;
    use crate::housedeck::HouseDeck;
    use crate::selfplay::{Caps, play, table_for};
    use baylee_core::preset::AIProfile;
    use std::time::Duration;

    /// A four-seat game of two sides converts: every decision has four seat
    /// rows, the deciding seat's deck list, and options its label is one of;
    /// the winners are one side, relative to each decision's seat.
    #[test]
    fn a_four_seat_team_game_converts() {
        let decks = ["allytifact", "victory", "schwarzrand", "weltenbaum"]
            .map(|k| HouseDeck::named(k).unwrap());
        let refs: Vec<&HouseDeck> = decks.iter().collect();
        let preset = table_for(7, &refs, &[AIProfile::STEADY; 4], &[1, 2, 1, 2]);
        let played = play(
            &preset,
            "convert3-7",
            Caps {
                answers: 80_000,
                wall: Duration::from_secs(300),
            },
        );
        assert!(played.outcome.finished(), "{:?}", played.outcome);
        let converted = convert(&played.record, Keep::default()).expect("the record converts");
        assert!(!converted.samples.is_empty());
        assert_eq!(converted.teams, [Some(1), Some(2), Some(1), Some(2)]);
        assert!(
            converted.winners == [0, 2] || converted.winners == [1, 3],
            "{:?}",
            converted.winners
        );
        let mut labelled = 0;
        for s in &converted.samples {
            assert_eq!(s.actor.seats.len(), 4);
            assert_eq!(s.actor.rows.len(), s.actor.cards.len());
            assert!(s.actor.rows.iter().all(|r| r.len() == ENT_COLS.len()));
            assert!(!s.actor.deck_cards.is_empty());
            assert_eq!(s.actor.offered_dropped, 0);
            if s.chosen >= 0 {
                labelled += 1;
                assert!((s.chosen as usize) < s.opts.len());
            }
            // The deciding seat's side, relative to it: itself and two on.
            assert_eq!(converted.team_mask(s.seat), 0b101);
            let won = converted.relative_mask(s.seat, &converted.winners);
            assert!(won == 0b101 || won == 0b1010, "{won:b}");
        }
        assert!(
            labelled * 10 > converted.samples.len() * 8,
            "{labelled} of {}",
            converted.samples.len()
        );
    }
}

/// Model preparation gate used by the replay converter before any sample exists.
fn encode_decision(view: &PlayerView, pending: &Pending, table: &Table<'_>) -> Option<Encoded> {
    policy::model_input_for_view(view, pending, || {
        encode(view, pending, &Picked::default(), table)
    })
}

#[cfg(test)]
mod damage_conversion_tests {
    use super::*;
    #[test]
    fn damage_decisions_never_produce_converter_features_or_samples() {
        let view = baylee_client_core::test_support::ViewBuilder::new(2).build();
        let table = Table {
            teams: &[],
            deck: &[],
        };
        let mut controlled = view.clone();
        controlled.decision_player = Some(baylee_core::ids::PlayerId::new(1));
        let ordinary = baylee_engine::choice::Pending::ChooseColor {
            player: baylee_core::ids::PlayerId::new(1),
            options: vec![baylee_core::mana::ManaColor::Red],
        };
        assert!(encode_decision(&controlled, &ordinary, &table).is_none());
        for pending in crate::damage_fixture::questions() {
            assert!(encode_decision(&view, &pending, &table).is_none());
        }
    }
}
