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
//! two seeds, and it is kept ([`Afterstate4::judged`]) only where
//!
//! - the fork's frame of the root is the replay's own,
//! - both forks give the same frame byte for byte,
//! - neither fork showed the seat a card it dealt, in a view of the seat's
//!   or in a reveal, and
//! - neither fork drew from its random stream (a coin, a die, a choice at
//!   random).
//!
//! The last two are not left to the frames: two deals hold the same card now
//! and then, and two flips fall alike every other time, and the frame would
//! then show one deal's card or flip as if every deal had it. A shuffle draws
//! too and shows nothing; it is refused with the rest, since a count of draws
//! cannot tell the two apart. An event that only moves a hidden card, an
//! opponent's draw above all, shows the seat no card and is kept
//! ([`Hidden4::event`] counts those). What reads a hidden card
//! without showing it, and a question another seat owes because of one, are
//! caught by the frames alone, where the two deals make them differ.
//!
//! Seeds are a function of the game's id in the dataset, the seat and the
//! root's cursor alone, so a capture is a function of its record and that id.

use std::collections::BTreeSet;

use baylee_core::ids::{ObjectId, PlayerId};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::{DeckKnowledge, DetectLoops, LoopDetection};
use baylee_engine::event::GameEvent;
use baylee_engine::zone::{Zone, ZoneLocation};
use baylee_gamehost::{Session, SimulationSession};
use baylee_view::PlayerView;

pub use super::search_roots::AfterCandidate4;
use crate::features4::{EncodedCandidates4, EncodedState4, PendingKind4, SeatKnowledge4};
use crate::netplay4::{Script4, planned_cast};

/// Where a candidate's afterstate stopped, or why the position it reached is
/// not kept, by the code the sidecar writes. A code never changes its meaning;
/// a new one is appended.
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
    /// The fork's frame of the root is not the replay's.
    RootMismatch = 7,
    /// The two forks' frames of the position differ.
    ForksDiffer = 8,
    /// The candidate's target question did not follow its answer, or the
    /// target is no answer to it.
    TargetRefused = 9,
    /// A fork showed the seat a card it dealt.
    DealtSeen = 10,
    /// A fork drew from its random stream.
    Chance = 11,
}

impl AfterStop4 {
    /// Every stop, in the order of the codes.
    pub const ALL: [Self; 12] = [
        Self::Question,
        Self::GameOver,
        Self::ForkRefused,
        Self::AnswerRefused,
        Self::ScriptMismatch,
        Self::ScriptBroken,
        Self::TooLong,
        Self::RootMismatch,
        Self::ForksDiffer,
        Self::TargetRefused,
        Self::DealtSeen,
        Self::Chance,
    ];

    /// A stable name for counts and the sidecar's header.
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
            Self::RootMismatch => "root_mismatch",
            Self::ForksDiffer => "forks_differ",
            Self::TargetRefused => "target_refused",
            Self::DealtSeen => "dealt_seen",
            Self::Chance => "chance",
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
    /// A digest of the cards the fork dealt into the zones the seat cannot
    /// see, in order: two forks with one digest were dealt the same cards
    /// (counts only). The fork's own hash would not say: it covers the random
    /// stream, which the fork's seed sets whatever the deal.
    pub deal: u64,
}

/// What became of hidden information between a root and a stop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hidden4 {
    /// Whether a hidden-information event (a card leaving a library, a
    /// reveal, a shuffle, a coin flip) happened in the first fork. For
    /// counts: it says nothing of what the seat was shown, as an opponent's
    /// draw is one.
    pub event: bool,
    /// Whether either fork showed the seat a card it dealt: in a view of the
    /// seat's, or in a reveal.
    pub dealt_seen: bool,
    /// Whether either fork drew from its random stream.
    pub chance: bool,
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
    /// What became of hidden information on the way.
    pub hidden: Hidden4,
    /// The frame of each fork, where it reached a position.
    pub frames: [Option<AfterFrame4>; 2],
}

impl Afterstate4 {
    /// Whether the afterstate is kept ([`AfterStop4::reached`]) or why not,
    /// the first fault in the module's order: `root_match` where the fork's
    /// frame of the root is the replay's, `forks_match` where both forks'
    /// frames are the same bytes.
    #[must_use]
    pub const fn judged(&self, root_match: bool, forks_match: bool) -> AfterStop4 {
        if !self.stop.reached() {
            self.stop
        } else if !root_match {
            AfterStop4::RootMismatch
        } else if !forks_match {
            AfterStop4::ForksDiffer
        } else if self.hidden.dealt_seen {
            AfterStop4::DealtSeen
        } else if self.hidden.chance {
            AfterStop4::Chance
        } else {
            self.stop
        }
    }
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

/// Whether a journal event moves or shows hidden information.
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
pub struct RootContext<'a, D: LoopDetection = DetectLoops> {
    /// The replay's table at the root.
    pub session: &'a Session<D>,
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
fn fork<D: LoopDetection>(
    root: &RootContext<'_, D>,
    prior: &baylee_engine::engine::PublicPrior,
    which: u64,
) -> Option<SimulationSession<D>> {
    root.session
        .fork_for(
            root.seat,
            DeckKnowledge::Prior(prior),
            fork_seed(root.game, root.seat.get(), root.cursor, which),
        )
        .ok()
}

/// What a fork dealt, and whether the seat has been shown any of it.
struct Dealt {
    /// The objects the fork dealt: a fork puts every card it deals into a new
    /// object (`Engine::fork_for`), and an object keeps its id as it moves.
    objects: BTreeSet<ObjectId>,
    /// [`AfterFrame4::deal`].
    digest: u64,
    /// How often the fork had drawn from its random stream at the root.
    draws: u64,
    /// The length of the fork's journal at the root.
    journal: usize,
    seen: bool,
}

impl Dealt {
    /// What `fork` dealt, before anything is played in it: every card of a
    /// library, of another seat's hand and of another seat's cards outside
    /// the game, but those `root`'s view shows (a hand or a library top a
    /// rule reveals, a teammate's hand), which a fork keeps.
    fn of<D: LoopDetection>(root: &RootContext<'_, D>, fork: &SimulationSession<D>) -> Self {
        let shown: BTreeSet<ObjectId> = root
            .view
            .identities()
            .filter_map(|seen| seen.object)
            .collect();
        let state = fork.state();
        let (mut objects, mut digest) = (BTreeSet::new(), 0);
        for player in &state.players {
            let others = [
                ZoneLocation::Hand(player.id),
                ZoneLocation::OutsideGame(player.id),
            ];
            let hidden = std::iter::once(ZoneLocation::Library(player.id))
                .chain(others.into_iter().filter(|_| player.id != root.seat));
            for zone in hidden {
                for &id in state.zones.list(zone) {
                    if shown.contains(&id) {
                        continue;
                    }
                    objects.insert(id);
                    let card = state.object(id).and_then(|object| object.card);
                    digest = mix(
                        digest,
                        card.map_or(0, |card| 1 + u64::from(card.index.get())),
                    );
                }
            }
        }
        Self {
            objects,
            digest,
            draws: state.rng.calls(),
            journal: state.journal.len(),
            seen: false,
        }
    }

    /// Notes what `view` shows the seat.
    fn watch(&mut self, view: &PlayerView) {
        self.seen |= view
            .identities()
            .filter_map(|seen| seen.object)
            .any(|id| self.objects.contains(&id));
    }

    /// Whether the seat was shown a dealt card: in a view it was watched
    /// through, or in a reveal, which every seat sees and which may be gone
    /// from the next view.
    fn shown<D: LoopDetection>(&self, fork: &SimulationSession<D>) -> bool {
        self.seen
            || self.events(fork).any(|event| {
                matches!(event, GameEvent::Revealed { cards, .. }
                    if cards.iter().any(|card| self.objects.contains(card)))
            })
    }

    /// What has happened in `fork` since the root.
    fn events<'a, D: LoopDetection>(
        &self,
        fork: &'a SimulationSession<D>,
    ) -> impl Iterator<Item = &'a GameEvent> {
        fork.state()
            .journal
            .entries()
            .get(self.journal..)
            .unwrap_or_default()
            .iter()
            .map(|entry| &entry.event)
    }
}

/// Plays `candidate` in `fork` as the search played it, up to its last
/// answer: (where it stopped short, or `Question` when it did not; answers
/// applied).
fn play<D: LoopDetection>(
    root: &RootContext<'_, D>,
    fork: &mut SimulationSession<D>,
    dealt: &mut Dealt,
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
    if let Some(target) = &candidate.target {
        if !answer_target(root.seat, fork, dealt, target) {
            return (AfterStop4::TargetRefused, answers);
        }
        answers += 1;
    }
    let Some(mut script) = script else {
        return (AfterStop4::Question, answers);
    };
    follow(root.seat, fork, dealt, &mut script, &mut answers)
}

/// Answers the seat's target question with `target`, as a search with target
/// candidates answers it (`netplay4::search`): whether the question is its
/// next one and takes the answer.
fn answer_target<D: LoopDetection>(
    seat: PlayerId,
    fork: &mut SimulationSession<D>,
    dealt: &mut Dealt,
    target: &PlayerAction,
) -> bool {
    let Some((pending, view)) = fork.view_for_with_context(seat) else {
        return false;
    };
    dealt.watch(&view);
    matches!(&pending, Pending::ChooseTargets { player, .. } if *player == seat)
        && pending.answer_fault(target).is_none()
        && fork.act_slice(seat, target.clone(), 0).is_ok()
}

/// The rest of a planned cast's script, answered from it while the seat is
/// asked its questions.
fn follow<D: LoopDetection>(
    seat: PlayerId,
    fork: &mut SimulationSession<D>,
    dealt: &mut Dealt,
    script: &mut Script4,
    answers: &mut u16,
) -> (AfterStop4, u16) {
    for _ in 0..MAX_ANSWERS {
        let Some((pending, view)) = fork.view_for_with_context(seat) else {
            return (AfterStop4::ScriptBroken, *answers);
        };
        dealt.watch(&view);
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
/// next input: its question's view where it owes one, else what it observes;
/// with whether it owes it and the next question's kind.
fn observe<D: LoopDetection>(
    root: &RootContext<'_, D>,
    fork: &SimulationSession<D>,
    dealt: &mut Dealt,
) -> (AfterFrame4, bool, u8) {
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
    dealt.watch(&view);
    let mut knowledge = root.knowledge.clone();
    knowledge.observe(&view);
    let state = encode_state(&view, None, &knowledge, root.teams);
    (
        AfterFrame4 {
            state,
            turn: view.turn,
            deal: dealt.digest,
        },
        asked,
        kind,
    )
}

/// One candidate played in one fork.
struct Played {
    stop: AfterStop4,
    answers: u16,
    asked_seat: bool,
    next_kind: u8,
    hidden: Hidden4,
    frame: Option<AfterFrame4>,
}

impl Played {
    /// A candidate that reached no position.
    const fn short(stop: AfterStop4, answers: u16) -> Self {
        Self {
            stop,
            answers,
            asked_seat: false,
            next_kind: u8::MAX,
            hidden: Hidden4 {
                event: false,
                dealt_seen: false,
                chance: false,
            },
            frame: None,
        }
    }
}

/// One candidate's afterstate in fork `which`.
fn one<D: LoopDetection>(
    root: &RootContext<'_, D>,
    prior: &baylee_engine::engine::PublicPrior,
    candidate: &AfterCandidate4,
    which: u64,
) -> Played {
    let Some(mut fork) = fork(root, prior, which) else {
        return Played::short(AfterStop4::ForkRefused, 0);
    };
    let mut dealt = Dealt::of(root, &fork);
    let (mut stop, answers) = play(root, &mut fork, &mut dealt, candidate);
    if !stop.reached() {
        return Played::short(stop, answers);
    }
    if matches!(fork.pending(), Pending::GameOver(_)) {
        stop = AfterStop4::GameOver;
    }
    let (frame, asked_seat, next_kind) = observe(root, &fork, &mut dealt);
    Played {
        stop,
        answers,
        asked_seat,
        next_kind,
        hidden: Hidden4 {
            event: dealt.events(&fork).any(hidden_event),
            dealt_seen: dealt.shown(&fork),
            chance: fork.state().rng.calls() != dealt.draws,
        },
        frame: Some(frame),
    }
}

/// The afterstates of every candidate at `root`, in their order, with the
/// first fork's frame of the root.
#[must_use]
pub fn capture<D: LoopDetection>(
    root: &RootContext<'_, D>,
    candidates: &[AfterCandidate4],
) -> RootAfterstates4 {
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
            let first = one(root, &prior, candidate, 0);
            let second = first
                .stop
                .reached()
                .then(|| one(root, &prior, candidate, 1));
            let of_second = second
                .as_ref()
                .map_or_else(Hidden4::default, |played| played.hidden);
            Afterstate4 {
                stop: first.stop,
                answers: first.answers,
                asked_seat: first.asked_seat,
                next_kind: first.next_kind,
                hidden: Hidden4 {
                    event: first.hidden.event,
                    dealt_seen: first.hidden.dealt_seen || of_second.dealt_seen,
                    chance: first.hidden.chance || of_second.chance,
                },
                frames: [first.frame, second.and_then(|played| played.frame)],
            }
        })
        .collect();
    RootAfterstates4 {
        root: root_frame,
        candidates,
    }
}

#[cfg(test)]
mod tests;
