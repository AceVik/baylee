//! Afterstates of a searched root (`search_afterstate_v1`, `convert4
//! --search-afterstates`): what the table looks like to the searching seat
//! right after each answer the search tried, for a net that judges an answer
//! by the position it leads to.
//!
//! At a root the replay's table is forked for the root's seat with
//! [`DeckKnowledge::Prior`]: every card the seat cannot see is removed unread
//! and dealt again from the seat's public prior, so no real hidden card is in
//! the fork at all. Every chair stays driven, so nobody but the capture
//! answers. The candidate's answer is applied as the search applied it; a
//! planned cast (the event's script) is then tapped for and cast by the
//! planner exactly as the search's macro candidate plays it
//! ([`crate::netplay4::planned_cast`]). The capture stops at the first
//! question after the candidate's last answer, whoever owes it.
//!
//! The afterstate is the seat's observation of that table, encoded as the
//! replay encodes an observation frame. It is made in two forks dealt from
//! two seeds, and the converter keeps it only where both forks give the same
//! frame byte for byte (a draw, a reveal or any other event that shows the
//! seat a dealt card makes them differ), and where the fork's frame of the
//! root is the replay's own. Seeds are a function of the game's id in the
//! dataset, the seat and the root's cursor alone, so a capture is a function
//! of its record and that id.

use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::DeckKnowledge;
use baylee_engine::event::GameEvent;
use baylee_engine::zone::Zone;
use baylee_gamehost::{Session, SimulationSession};
use baylee_view::PlayerView;

pub use super::search_roots::AfterCandidate4;
use crate::features4::{EncodedCandidates4, EncodedState4, PendingKind4, SeatKnowledge4};
use crate::netplay4::{Script4, planned_cast};

/// Why a candidate's afterstate stopped where it did, by code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum AfterStop4 {
    /// At the next question after the candidate's last answer.
    Question = 0,
    /// The game ended.
    GameOver = 1,
    /// The table would not fork.
    ForkRefused = 2,
    /// The fork refused one of the candidate's answers.
    AnswerRefused = 3,
    /// The event's planned candidate is not the planner's first tap for its
    /// cast, or the planner has no plan for it.
    ScriptMismatch = 4,
    /// The planned cast's script ended before its cast.
    ScriptBroken = 5,
    /// More answers than a candidate can have.
    TooLong = 6,
}

impl AfterStop4 {
    /// A stable name for counts.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Question => "question",
            Self::GameOver => "game_over",
            Self::ForkRefused => "fork_refused",
            Self::AnswerRefused => "answer_refused",
            Self::ScriptMismatch => "script_mismatch",
            Self::ScriptBroken => "script_broken",
            Self::TooLong => "too_long",
        }
    }

    /// Whether the fork reached a position to encode.
    #[must_use]
    pub const fn reached(self) -> bool {
        matches!(self, Self::Question | Self::GameOver)
    }
}

/// One fork's afterstate of one candidate.
#[derive(Clone, Debug)]
pub struct AfterFrame4 {
    /// The seat's observation, encoded as an observation frame's state.
    pub state: EncodedState4,
    /// The turn the frame is in.
    pub turn: u32,
    /// The fork's determinism hash at the stop, hidden cards included: two
    /// forks dealt differently differ here even where the seat's frames do
    /// not (counts only).
    pub hash: u64,
}

/// One candidate's afterstate in both forks.
#[derive(Clone, Debug)]
pub struct Afterstate4 {
    /// Where the first fork stopped.
    pub stop: AfterStop4,
    /// The answers the first fork applied for the candidate.
    pub answers: u16,
    /// Whether the next question is the seat's own.
    pub asked_seat: bool,
    /// The next question's [`PendingKind4`] code; `u8::MAX` without one.
    pub next_kind: u8,
    /// Whether a hidden-information event (a card leaving a library, a
    /// reveal, a shuffle, a coin flip) happened in the first fork between
    /// the root and the stop. For counts: the frames' equality decides.
    pub hidden_event: bool,
    /// The frame of each fork, where it reached a position.
    pub frames: [Option<AfterFrame4>; 2],
}

/// The afterstates of one root, with the first fork's frame of the root.
#[derive(Clone, Debug)]
pub struct RootAfterstates4 {
    /// The first fork's encoding of the root (state, then menu), to be
    /// compared with the replay's own frame; `None` where it would not fork.
    pub root: Option<(EncodedState4, EncodedCandidates4)>,
    /// One per candidate, in the event's order.
    pub candidates: Vec<Afterstate4>,
}

/// The most answers one candidate may take (its taps, colours and cast).
const MAX_ANSWERS: usize = 64;

/// `SplitMix64` of a seed and a stream.
fn mix(seed: u64, stream: u64) -> u64 {
    let mut z = seed ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Purpose tag of the capture's fork seeds.
const AFTERSTATE: u64 = u64::from_be_bytes(*b"aftrstat");

/// The seed of fork `which` (0 or 1) at the root of `seat` at `cursor` in
/// game `game`.
#[must_use]
pub fn fork_seed(game: u32, seat: u8, cursor: u32, which: u64) -> u64 {
    let root = (u64::from(game) << 40) ^ (u64::from(seat) << 32) ^ u64::from(cursor);
    mix(mix(AFTERSTATE ^ root, which + 1), which + 1)
}

/// Whether a journal event can show a seat a card it could not see before.
fn hidden_event(event: &GameEvent) -> bool {
    matches!(
        event,
        GameEvent::Revealed { .. } | GameEvent::Shuffled { .. } | GameEvent::CoinFlipped { .. }
    ) || matches!(
        event,
        GameEvent::ZoneChanged {
            from: Zone::Library,
            ..
        }
    )
}

/// What the replay knows at a root, for its afterstates.
pub struct RootContext<'a> {
    /// The replay's table at the root.
    pub session: &'a Session,
    /// The root's seat.
    pub seat: PlayerId,
    /// The seat's view of the root (with context).
    pub view: &'a PlayerView,
    /// The root's question.
    pub pending: &'a Pending,
    /// The seat's knowledge, having observed the root.
    pub knowledge: &'a SeatKnowledge4,
    /// The table's teams.
    pub teams: &'a [Option<u8>],
    /// The game's id in the dataset and the root frame's cursor (seeds).
    pub game: u32,
    /// The root frame's cursor.
    pub cursor: u32,
}

/// Encodes a state as the replay does for encoder 10 (`encode_state7`).
fn encode_state(
    view: &PlayerView,
    pending: Option<&Pending>,
    knowledge: &SeatKnowledge4,
    teams: &[Option<u8>],
) -> EncodedState4 {
    crate::features7::encode_state7(view, pending, Some(knowledge), teams)
}

/// The fork of `root`'s table for fork `which`.
fn fork(
    root: &RootContext<'_>,
    prior: &baylee_engine::engine::PublicPrior,
    which: u64,
) -> Option<SimulationSession> {
    root.session
        .fork_for(
            root.seat,
            DeckKnowledge::Prior(prior),
            fork_seed(root.game, root.seat.get(), root.cursor, which),
        )
        .ok()
}

/// Plays `candidate` in `fork` as the search played it, up to its last
/// answer: (where it stopped short, or `Question` when it did not; answers
/// applied).
fn play(
    root: &RootContext<'_>,
    fork: &mut SimulationSession,
    candidate: &AfterCandidate4,
) -> (AfterStop4, u16) {
    let script = match &candidate.script {
        None => None,
        Some(PlayerAction::CastSpell { card }) => {
            let Pending::Priority { legal, .. } = root.pending else {
                return (AfterStop4::ScriptMismatch, 0);
            };
            match planned_cast(root.view, legal, *card) {
                Some((first, script)) if first == candidate.action => Some(script),
                _ => return (AfterStop4::ScriptMismatch, 0),
            }
        }
        Some(_) => return (AfterStop4::ScriptMismatch, 0),
    };
    if fork
        .act_slice(root.seat, candidate.action.clone(), 0)
        .is_err()
    {
        return (AfterStop4::AnswerRefused, 0);
    }
    let mut answers = 1_u16;
    let Some(mut script) = script else {
        return (AfterStop4::Question, answers);
    };
    follow(root.seat, fork, &mut script, &mut answers)
}

/// The rest of a planned cast's script, answered from it while the seat is
/// asked its questions.
fn follow(
    seat: PlayerId,
    fork: &mut SimulationSession,
    script: &mut Script4,
    answers: &mut u16,
) -> (AfterStop4, u16) {
    for _ in 0..MAX_ANSWERS {
        let Some((pending, view)) = fork.view_for_with_context(seat) else {
            return (AfterStop4::ScriptBroken, *answers);
        };
        let step = script.next(&view, &pending);
        let Some(action) = step.action().cloned() else {
            return (AfterStop4::ScriptBroken, *answers);
        };
        if fork.act_slice(seat, action, 0).is_err() {
            return (AfterStop4::AnswerRefused, *answers);
        }
        *answers += 1;
        if step.ends() {
            return (AfterStop4::Question, *answers);
        }
    }
    (AfterStop4::TooLong, *answers)
}

/// The seat's observation of `fork` as the replay would encode it at the
/// next input: its question's view where it owes one, else what it observes.
fn observe(root: &RootContext<'_>, fork: &SimulationSession) -> (AfterFrame4, bool, u8) {
    let (view, asked, kind) = if let Some((pending, view)) = fork.view_for_with_context(root.seat) {
        (view, true, PendingKind4::of(&pending) as u8)
    } else {
        let kind = if matches!(fork.pending(), Pending::GameOver(_)) {
            u8::MAX
        } else {
            PendingKind4::of(fork.pending()) as u8
        };
        (fork.observe_view_with_context(root.seat), false, kind)
    };
    let mut knowledge = root.knowledge.clone();
    knowledge.observe(&view);
    let state = encode_state(&view, None, &knowledge, root.teams);
    (
        AfterFrame4 {
            state,
            turn: view.turn,
            hash: fork.snapshot_hash(),
        },
        asked,
        kind,
    )
}

/// One candidate's afterstate in fork `which`: (stop, answers, asked, next
/// kind, hidden event, frame).
fn one(
    root: &RootContext<'_>,
    prior: &baylee_engine::engine::PublicPrior,
    candidate: &AfterCandidate4,
    which: u64,
) -> (AfterStop4, u16, bool, u8, bool, Option<AfterFrame4>) {
    let Some(mut fork) = fork(root, prior, which) else {
        return (AfterStop4::ForkRefused, 0, false, u8::MAX, false, None);
    };
    let before = fork.state().journal.len();
    let (mut stop, answers) = play(root, &mut fork, candidate);
    if !stop.reached() {
        return (stop, answers, false, u8::MAX, false, None);
    }
    if matches!(fork.pending(), Pending::GameOver(_)) {
        stop = AfterStop4::GameOver;
    }
    let hidden = fork
        .state()
        .journal
        .entries()
        .get(before..)
        .unwrap_or_default()
        .iter()
        .any(|entry| hidden_event(&entry.event));
    let (frame, asked, kind) = observe(root, &fork);
    (stop, answers, asked, kind, hidden, Some(frame))
}

/// The afterstates of every candidate at `root`, in their order, with the
/// first fork's frame of the root.
#[must_use]
pub fn capture(root: &RootContext<'_>, candidates: &[AfterCandidate4]) -> RootAfterstates4 {
    let prior = root.session.public_prior(root.view);
    let root_frame = fork(root, &prior, 0).and_then(|fork| {
        let (pending, view) = fork.view_for_with_context(root.seat)?;
        let state = encode_state(&view, Some(&pending), root.knowledge, root.teams);
        let cands = crate::features10::encode_candidates10(&state, &view, &pending);
        Some((state, cands))
    });
    let candidates = candidates
        .iter()
        .map(|candidate| {
            let (stop, answers, asked_seat, next_kind, hidden_event, first) =
                one(root, &prior, candidate, 0);
            let second = if stop.reached() {
                one(root, &prior, candidate, 1).5
            } else {
                None
            };
            Afterstate4 {
                stop,
                answers,
                asked_seat,
                next_kind,
                hidden_event,
                frames: [first, second],
            }
        })
        .collect();
    RootAfterstates4 {
        root: root_frame,
        candidates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_are_a_function_of_the_root_and_differ_between_forks() {
        assert_eq!(fork_seed(7, 1, 300, 0), fork_seed(7, 1, 300, 0));
        assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(7, 1, 300, 1));
        assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(7, 0, 300, 0));
        assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(8, 1, 300, 0));
        assert_ne!(fork_seed(7, 1, 300, 0), fork_seed(7, 1, 301, 0));
    }

    #[test]
    fn only_a_reached_position_is_encoded() {
        assert!(AfterStop4::Question.reached() && AfterStop4::GameOver.reached());
        for stop in [
            AfterStop4::ForkRefused,
            AfterStop4::AnswerRefused,
            AfterStop4::ScriptMismatch,
            AfterStop4::ScriptBroken,
            AfterStop4::TooLong,
        ] {
            assert!(!stop.reached(), "{}", stop.key());
        }
    }

    #[test]
    fn a_card_leaving_a_library_is_a_hidden_event_and_a_life_change_is_not() {
        let draw = GameEvent::ZoneChanged {
            object: baylee_core::ids::ObjectId::new(3, 0),
            from: Zone::Library,
            to: Zone::Hand,
            cause: baylee_engine::event::Cause::TurnBased,
            place: None,
        };
        assert!(hidden_event(&draw));
        let played = GameEvent::ZoneChanged {
            object: baylee_core::ids::ObjectId::new(3, 0),
            from: Zone::Hand,
            to: Zone::Battlefield,
            cause: baylee_engine::event::Cause::Spell,
            place: None,
        };
        assert!(!hidden_event(&played));
        assert!(hidden_event(&GameEvent::Shuffled {
            player: PlayerId::new(0),
            zone: Zone::Library,
        }));
    }

    /// A house self-play record and, for each of seat 0's inputs, the
    /// candidates asked for: its recorded answer, a pass, a land that is no
    /// card of the game, and a planned "cast" that is no cast.
    fn game_and_roots() -> (Vec<u8>, super::super::search_roots::Roots4) {
        use crate::housedeck::HouseDeck;
        use crate::selfplay::{Caps, play_with_options, table};
        let a = HouseDeck::named("allytifact").unwrap();
        let b = HouseDeck::named("victory").unwrap();
        let preset = table(23, &a, &b, [baylee_core::preset::AIProfile::SHARP; 2]);
        let played = play_with_options(
            &preset,
            "t-afterstates-23",
            Caps {
                answers: 10_000,
                wall: std::time::Duration::from_secs(60),
            },
            true,
        );
        assert!(played.outcome.finished(), "{:?}", played.outcome);
        let mut afterstates = std::collections::BTreeMap::new();
        for line in played
            .record
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
        {
            if let Ok(baylee_gamehost::record::Line::Input {
                seat: 0, n, action, ..
            }) = serde_json::from_slice(line)
            {
                let plain = |action: PlayerAction| AfterCandidate4 {
                    action,
                    script: None,
                };
                afterstates.insert(
                    (0, n),
                    vec![
                        plain(action),
                        plain(PlayerAction::PassPriority),
                        plain(PlayerAction::PlayLand {
                            card: baylee_core::ids::ObjectId::new(9_999, 0),
                        }),
                        AfterCandidate4 {
                            action: PlayerAction::PassPriority,
                            script: Some(PlayerAction::PassPriority),
                        },
                    ],
                );
            }
        }
        let roots = super::super::search_roots::Roots4 {
            roots: afterstates.keys().copied().collect(),
            menu: |_, _, own, _| (own.clone(), own.candidates.len()),
            afterstates,
        };
        (played.record, roots)
    }

    /// Every root the replay captured, with its frame: (root frame,
    /// afterstates).
    fn replay(
        record: &[u8],
        roots: &super::super::search_roots::Roots4,
    ) -> Vec<(Step4, RootAfterstates4)> {
        let mut out = Vec::new();
        super::super::stream_game4_with_roots(
            record,
            5,
            true,
            None,
            None,
            None,
            false,
            10,
            None,
            super::super::TeamHands::Private,
            Some(roots),
            |mut step| {
                if let Some(root) = step.search_root.take() {
                    let captured = root.afterstates.clone().expect("asked for afterstates");
                    out.push((step, captured));
                }
                Ok(())
            },
        )
        .unwrap();
        out
    }

    use super::super::Step4;

    fn json<T: serde::Serialize>(value: &T) -> String {
        serde_json::to_string(value).unwrap()
    }

    #[test]
    fn afterstates_are_a_function_of_the_record_and_stop_at_the_next_question() {
        let (record, roots) = game_and_roots();
        let first = replay(&record, &roots);
        let again = replay(&record, &roots);
        assert!(first.len() > 10, "a game has priority roots of seat 0");
        assert_eq!(first.len(), again.len());
        let (mut passes_to_other, mut own_answers, mut unforked) = (0, 0, 0);
        for ((step, a), (_, b)) in first.iter().zip(&again) {
            assert_eq!(a.candidates.len(), 4);
            for (x, y) in a.candidates.iter().zip(&b.candidates) {
                assert_eq!(
                    (x.stop, x.answers, x.asked_seat, x.next_kind),
                    (y.stop, y.answers, y.asked_seat, y.next_kind)
                );
                for (p, q) in x.frames.iter().zip(&y.frames) {
                    assert_eq!(
                        p.as_ref().map(|f| json(&f.state)),
                        q.as_ref().map(|f| json(&f.state))
                    );
                }
            }
            // Mid-chain (mana floating, a cast under way) the engine will not
            // fork: nothing is played or encoded there.
            let Some((state, cands)) = a.root.as_ref() else {
                assert!(
                    a.candidates
                        .iter()
                        .all(|c| c.stop == AfterStop4::ForkRefused
                            && c.frames.iter().all(Option::is_none))
                );
                unforked += 1;
                continue;
            };
            // The fork's frame of the root is the replay's own.
            assert_eq!(
                json(state),
                json(&step.state),
                "root at {}",
                step.step_cursor
            );
            assert_eq!(cands.candidates.len(), step.cand_feat.len());
            // The recorded answer is the table's: it reaches the next question
            // after one answer, as do both forks.
            let own = &a.candidates[0];
            assert!(own.stop.reached(), "{:?} at {}", own.stop, step.step_cursor);
            assert_eq!(own.answers, 1);
            assert!(own.frames.iter().all(Option::is_some));
            own_answers += 1;
            // A pass with priority moves the question on; where it goes to the
            // other seat, the seat is not asked.
            let pass = &a.candidates[1];
            if pass.stop.reached() && !pass.asked_seat {
                passes_to_other += 1;
            }
            // No such land: the fork refuses it and nothing is encoded.
            let land = &a.candidates[2];
            assert_eq!(land.stop, AfterStop4::AnswerRefused);
            assert!(land.frames.iter().all(Option::is_none));
            // A script that is no planned cast is never played.
            assert_eq!(a.candidates[3].stop, AfterStop4::ScriptMismatch);
            assert_eq!(a.candidates[3].answers, 0);
        }
        assert!(
            own_answers > 10 && passes_to_other > 0,
            "{own_answers} {passes_to_other} {unforked}"
        );
    }
}
