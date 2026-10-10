//! One game session: an engine plus seats (humans and AI). Socket-free so
//! tests and both servers (engine-server dev harness, gateway) drive it
//! directly; transport lives with the callers.

use std::collections::BTreeMap;

use baylee_ai::{HeuristicAgent, policy_seed};
use baylee_cards::dsl::CardDef;
use baylee_core::ids::{CardIndex, PlayerId, SeatSet};
use baylee_core::preset::{GamePreset, HouseRules, PrintInfo, SeatController};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::state::CardLookup;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{ClockAnswer, HouseAnswer, LOG_TAIL_CAP, LogTail, PlayerView, PolicyAct};

use crate::log::GameLog;
use crate::record::{Mind, MindKind, Recorder, Source};

mod answers;
mod chairs;
mod clocks;
mod frames;
mod pump;
mod resume;
mod spectators;

/// How many of its policies' answers a seat's view carries (#234): the
/// latest, so a seat yielding to a long loop is not sent the loop. Their
/// numbers tell a client what it missed.
const POLICY_WINDOW: usize = 16;

/// Registry lookup backed by the compiled card pool.
pub struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        baylee_cards::by_index(index)
    }
    fn token_id(&self, token: &baylee_cards_dsl::TokenDef) -> Option<u16> {
        let id = baylee_cards::tokens::token_id(token);
        baylee_cards::tokens::by_token_id(id).map(|_| id)
    }
}

/// What sits in a seat.
#[derive(Clone, Debug)]
pub enum SeatKind {
    /// A human connection (any number of these per game).
    Human,
    /// An auto-driven AI seat.
    Ai(HeuristicAgent),
    /// An AI seat whose controls somebody has taken.
    ///
    /// It answers over a socket exactly as a human seat does — that *is* the
    /// feature: the dev harness can play the opponent by hand, so a client
    /// change can be exercised against a chosen line instead of against
    /// whatever the heuristic happened to pick. The agent is kept rather than
    /// dropped, so releasing hands the chair back to the house AI in the
    /// middle of a game.
    ///
    /// Reachable only from the loopback dev harness. Nothing in the gateway
    /// path constructs one, and it must stay that way: a seat someone else
    /// can take over is an opponent someone else can play.
    Driven(HeuristicAgent),
    /// A player's chair the house is holding, because nobody is on the other
    /// end of it.
    ///
    /// The mirror of [`SeatKind::Driven`], and the reason both exist: a chair
    /// and whoever is answering for it are two different things. A seat with
    /// no socket is sent nothing and is on no decision clock, which is right
    /// — and left the whole table waiting on a player who had closed their
    /// laptop. `HouseRules::reconnect_window_secs` says how long to wait
    /// before the house sits down instead; the player gets the chair back the
    /// moment their socket returns.
    StandIn(HeuristicAgent),
}

impl SeatKind {
    /// Whether this seat's answers arrive over a socket rather than from an
    /// agent inside the process.
    ///
    /// The distinction [`Session::pump`] turns on, and the reason it is a
    /// method: "is it a human" and "does it answer over the wire" were the
    /// same question until a seat could be taken over, and reading it as the
    /// former in even one place would leave a driven seat being played by the
    /// house AI it was taken from.
    #[must_use]
    pub const fn answers_over_socket(&self) -> bool {
        matches!(self, Self::Human | Self::Driven(_))
    }

    /// Whether the chair is an AI chair, however it is being played.
    ///
    /// What the roster shows, so a seat does not change its name in the
    /// lobby the moment a developer takes the controls.
    #[must_use]
    pub const fn is_ai_chair(&self) -> bool {
        matches!(self, Self::Ai(_) | Self::Driven(_))
    }

    /// Whether this is a player's chair the house is currently holding.
    ///
    /// A third answer rather than a second reading of `is_ai_chair`, because
    /// the chair has not changed hands — only who is answering for it has.
    #[must_use]
    pub const fn is_away(&self) -> bool {
        matches!(self, Self::StandIn(_))
    }
}

/// A live game: engine plus the seat roster.
pub struct Session {
    engine: Engine<RegistryLookup>,
    seats: Vec<SeatKind>,
    scouting_decks: Vec<baylee_ai::intelligence::DeckIntel>,
    seq: u64,
    /// How many times the *question* has changed — see
    /// [`Session::decision_seq`].
    decisions: u64,
    /// Kept for the decision clock; the engine has its own copy for rules.
    house_rules: HouseRules,
    /// Per seat, the [`Session::decision_seq`] at which it was asked the
    /// question it owes now: the anchor its decision clock runs against
    /// ([`Session::asked_at`]).
    asked_at: Vec<u64>,
    /// Per seat, the last decision-clock reading the caller took, and the
    /// question it was taken for.
    ///
    /// A reading rather than a clock: this crate may not own one, so the
    /// number arrives from outside and is stored with the `decision_seq` it
    /// was true of. That anchor is what makes a stale value harmless —
    /// [`Session::decision_remaining_ms`] discards it once the question has
    /// moved on, rather than letting the previous question's leftovers run
    /// against this one.
    ///
    /// Two nested `Option`s, and they are not the same question. The outer
    /// one is *whether anybody has said anything about this question yet*;
    /// the inner one is the caller saying **no clock is running** — an
    /// untimed table, an AI chair, or a seat waiting out its reconnect
    /// window. One `Option` conflated them, and the cost was exact: nothing
    /// has been read when a game starts, so the opening question of every
    /// game came out as "no clock" and was the one decision nobody was shown
    /// a countdown for.
    clock: Vec<Option<(u64, Option<u32>)>>,
    /// The print table the game was built from.
    ///
    /// The rules kernel has no use for it; a client has nothing without it.
    /// It is the only thing that turns a `PrintRef` into a card face, and a
    /// networked client never sees the preset it came from.
    prints: Vec<PrintInfo>,
    /// Which print table entries each seat has been shown.
    ///
    /// The table is shared by the whole game and deduplicated per card, so a
    /// seat handed all of it would be handed the union of every deck at the
    /// table — the one piece of hidden information with no game object to hide
    /// behind. A seat starts entitled to its own deck's printings, which it
    /// already knows, and earns the rest by seeing the cards.
    ///
    /// Not game state: it never enters the engine, the journal, or a snapshot
    /// hash. It is what a seat has been *told*, which is a property of the
    /// connection, not of the game.
    revealed: Vec<Vec<bool>>,
    /// Team per seat, for the seat roster.
    teams: Vec<Option<u8>>,
    /// Per seat, who answered its most recent decision in its place — the
    /// decision clock or a stand-in — and `None` when the seat answered it
    /// itself. What [`SeatView::house_answered`](baylee_view::SeatView::house_answered)
    /// reports.
    ///
    /// Not game state: the engine takes an action without asking who produced
    /// it, and should — a rules kernel with two doors would have two sets of
    /// rules. Only the host knows, so the host records it: set by
    /// [`Session::answer_by_clock`] and by [`Session::pump`] for a stand-in,
    /// cleared by the seat's own decision through [`Session::act`]. Never
    /// set for an AI chair, driven or not: the roster already says the house
    /// plays it.
    house_answered: Vec<Option<HouseAnswer>>,
    /// Per seat, what its own per-ability policies answered for it since it
    /// last answered by hand, oldest first, the latest [`POLICY_WINDOW`]
    /// (#234). What [`PlayerView::policy_acts`] reports.
    ///
    /// Not game state, for `house_answered`'s reason: the engine journals
    /// each answer its policy gave, but only the host knows which answers the
    /// seat gave by hand. Read off the journal after every applied answer,
    /// cleared by the seat's own decision through [`Session::act`].
    policy_acts: Vec<Vec<PolicyAct>>,
    /// Per seat, how many answers its policies have given, which numbers the
    /// next one.
    policy_counted: Vec<u32>,
    /// How much of the journal the policy windows have read.
    policy_read: usize,
    /// Which seats have yet to be told that a chair changed hands.
    ///
    /// Not game state, for the same reason `revealed` is not: it is what a
    /// seat has been *told*, which is a property of the connection. See
    /// [`Session::roster_changed`].
    roster_dirty: Vec<bool>,
    /// What clients call this game (see [`Session::describe`]).
    game_id: String,
    /// What clients call each seat (see [`Session::describe`]).
    names: Vec<String>,
    /// The game log, kept once for the whole table and told to each seat as
    /// it may know it (#262).
    log: GameLog,
    /// Per seat, how many of the log's lines it has been sent.
    ///
    /// Not game state, for the reason `revealed` is not: it is what a seat
    /// has been *told*. It starts again at 0 when a socket attaches
    /// ([`Session::retell_log`]).
    told: Vec<usize>,
    /// Per seat, the teammates it shows its hand to (#265).
    ///
    /// Not game state: showing a hand moves nothing in the game, so it never
    /// enters the engine, the journal or the snapshot hash. It is what a
    /// seat *wants*, and who is actually shown the hand is read afresh for
    /// every view ([`Session::shows_hand`]), so a seat that left the game is
    /// shown nothing more without this having to be told.
    showing: Vec<SeatSet>,
    /// Per seat, the teammates asking to see its hand, not yet answered.
    asking: Vec<SeatSet>,
    /// `(owner, asker)` → the turn the owner turned the asker down in. The
    /// asker may ask again from the next turn on.
    declined: BTreeMap<(u8, u8), u32>,
    /// The game's record (#315), when the host asked for one
    /// ([`Session::new_recorded`]); `None` in the client's own table, which
    /// has nowhere to keep it.
    record: Option<Recorder>,
    /// Per seat, what its client last declared answers it, as the record
    /// has it ([`Session::declare_mind`]); `None` while nothing was.
    ///
    /// Not game state: it is what a client *said*, for the record's reader,
    /// and no seat is ever shown it.
    minds: Vec<Option<Mind>>,
    /// Per seat, whether a socket opened since its client last declared
    /// ([`Session::socket_opened`]).
    unsaid: Vec<bool>,
    /// Per seat, how many declarations the record holds; past
    /// [`MAX_DECLARATIONS`] the rest are refused.
    declarations: Vec<u32>,
    /// What the spectators have been told, together (`spectators.rs`).
    gallery: spectators::Gallery,
}

/// The most declared minds one seat's record holds: a client that changes
/// its answer back and forth cannot grow the record without bound.
pub const MAX_DECLARATIONS: u32 = 256;

/// A seat's number as the policy-seed derivation takes it.
///
/// A preset carries at most eight seats and [`PlayerId`] is a `u8`, so this
/// cannot lose anything; it is written as a saturating conversion rather
/// than a cast so that nothing silently wraps if that ever stops being true.
fn seat_byte(index: usize) -> u8 {
    u8::try_from(index).unwrap_or(u8::MAX)
}

impl Session {
    /// Starts a game from a preset; Open seats become humans, AI seats
    /// get a heuristic agent.
    #[must_use]
    pub fn new(preset: &GamePreset) -> Option<Self> {
        let seats: Vec<SeatKind> = preset
            .seats
            .iter()
            .enumerate()
            .map(|(i, s)| match &s.controller {
                SeatController::Ai(profile) => Some(SeatKind::Ai(
                    HeuristicAgent::new(*profile)
                        // Not `preset.seed`: that stream dealt the hands.
                        // A table nobody has named yet gets the derivation
                        // with no identifier, which is still independent of
                        // the shuffle; `describe` supplies the real one.
                        .with_seed(policy_seed("", seat_byte(i)))
                        .with_teams(preset.seats.iter().map(|s| s.team).collect()),
                )),
                _ => Some(SeatKind::Human),
            })
            .collect::<Option<_>>()?;
        let engine = Engine::new(preset, RegistryLookup).ok()?;
        let log = GameLog::new(engine.state());
        Some(Self {
            engine,
            seats,
            scouting_decks: crate::scouting::decks(preset),
            seq: 0,
            decisions: 0,
            house_rules: preset.house_rules.clone(),
            asked_at: vec![0; preset.seats.len()],
            clock: vec![None; preset.seats.len()],
            prints: preset.prints.clone(),
            revealed: preset
                .seats
                .iter()
                .map(|spec| own_prints(spec, preset.prints.len()))
                .collect(),
            teams: preset.seats.iter().map(|s| s.team).collect(),
            house_answered: vec![None; preset.seats.len()],
            policy_acts: vec![Vec::new(); preset.seats.len()],
            policy_counted: vec![0; preset.seats.len()],
            policy_read: 0,
            roster_dirty: vec![false; preset.seats.len()],
            game_id: String::new(),
            names: Vec::new(),
            log,
            told: vec![0; preset.seats.len()],
            showing: vec![SeatSet::new(); preset.seats.len()],
            asking: vec![SeatSet::new(); preset.seats.len()],
            declined: BTreeMap::new(),
            record: None,
            minds: vec![None; preset.seats.len()],
            unsaid: vec![false; preset.seats.len()],
            declarations: vec![0; preset.seats.len()],
            gallery: spectators::Gallery {
                revealed: vec![false; preset.prints.len()],
                ..spectators::Gallery::default()
            },
        })
    }

    /// [`Session::new`], keeping the game's record (#315) as it goes: every
    /// input the engine takes, in order, which [`Session::take_record`]
    /// hands out. `build` names the build playing it.
    #[must_use]
    pub fn new_recorded(preset: &GamePreset, build: &str) -> Option<Self> {
        let mut session = Self::new(preset)?;
        session.record = Some(Recorder::new(preset, build, session.engine.snapshot_hash()));
        Some(session)
    }

    /// The record written since the last take, whole lines only; empty when
    /// nothing was written or the session keeps no record.
    pub fn take_record(&mut self) -> Vec<u8> {
        self.record.as_mut().map(Recorder::take).unwrap_or_default()
    }

    /// How many bytes of record are waiting to be taken.
    #[must_use]
    pub fn record_pending(&self) -> usize {
        self.record.as_ref().map_or(0, Recorder::pending)
    }

    /// Writes down an action the engine has just applied, and the end of the
    /// game the first time the engine says it is over.
    fn recorded(&mut self, seat: PlayerId, by: Source, action: PlayerAction) {
        let Some(record) = self.record.as_mut() else {
            return;
        };
        // A socket that answers before it said what it is: what an earlier
        // socket declared does not hold for it.
        let at = seat.get() as usize;
        if by == Source::Seat && self.unsaid.get(at).copied().unwrap_or(false) {
            self.unsaid[at] = false;
            if self.minds[at]
                .as_ref()
                .is_some_and(|m| m.kind != MindKind::Undeclared)
            {
                record.mind(seat, Mind::undeclared());
                self.minds[at] = Some(Mind::undeclared());
            }
        }
        record.input(seat, by, action, &self.engine);
        if !record.ended()
            && let Pending::GameOver(result) = self.engine.pending()
        {
            let winners = self
                .winning_seats(*result)
                .iter()
                .map(|p| p.get())
                .collect();
            let reason = format!("{:?}", result.reason);
            if let Some(record) = self.record.as_mut() {
                record.end(winners, reason);
            }
        }
    }
}

/// The printings a seat already knows because they are its own.
///
/// A player has seen their own decklist; nothing is revealed by handing it
/// back. Everything outside this set has to be earned by seeing a card.
/// What the decision clock does with a question: tries the answer that does
/// nothing ([`baylee_engine::choice::timeout_answer`]), and asks the house
/// only where there is none or the engine refused it.
///
/// The refusal is not reachable with today's pool: the answers that do
/// nothing carry what the question says must be done — the creatures that
/// attack if able (CR 508.1d), the question's `obeying` blocks (CR 509.1c)
/// — and the engine accepts them. A requirement a question does not state
/// would make one refusable again, and a clock that stopped at the refusal
/// would ask the same seat the same question forever. So the order lives
/// here, apart from the session, where a refusing engine can be written as
/// a closure.
fn by_clock<S, T>(
    session: &mut S,
    pending: &Pending,
    answer: impl Fn(&mut S, PlayerAction) -> Result<T, String>,
    house: impl FnOnce(&S) -> Option<PlayerAction>,
) -> Option<Result<T, String>> {
    if let Some(nothing) = baylee_engine::choice::timeout_answer(pending)
        && let Ok(out) = answer(session, nothing)
    {
        return Some(Ok(out));
    }
    let action = house(session)?;
    Some(answer(session, action))
}

fn own_prints(spec: &baylee_core::preset::SeatSpec, len: usize) -> Vec<bool> {
    let mut shown = vec![false; len];
    let entries = spec
        .deck
        .iter()
        .chain(&spec.sideboard)
        .chain(spec.starting_hand.iter().flatten())
        .chain(&spec.starting_battlefield);
    for entry in entries {
        if let Some(slot) = shown.get_mut(entry.print.get() as usize) {
            *slot = true;
        }
    }
    shown
}

/// What the log needs to know of an answer before it is spent.
struct Asked {
    /// The answering seat's hand.
    hand: usize,
    /// Whether it is a mulligan.
    took: bool,
    /// How many cards it puts on the bottom, as a mulligan's keep does.
    bottomed: usize,
}

/// What a decision clock answered, for the log: the answer that does
/// nothing, by name, or a choice the house made because there was none.
fn clock_answer(pending: Option<&Pending>, action: &PlayerAction) -> ClockAnswer {
    let quiet = pending.and_then(baylee_engine::choice::timeout_answer);
    if quiet.as_ref() != Some(action) {
        return ClockAnswer::ChosenForThem;
    }
    match action {
        PlayerAction::PassPriority => ClockAnswer::Passed,
        PlayerAction::MulliganKeep => ClockAnswer::Kept,
        PlayerAction::DeclareAttackers { .. } => ClockAnswer::NoAttackers,
        PlayerAction::DeclareBlockers { .. } => ClockAnswer::NoBlockers,
        PlayerAction::YesNo(false) => ClockAnswer::Declined,
        _ => ClockAnswer::ChosenForThem,
    }
}

/// A seat's view and its log tail, as the frames that carry them.
///
/// Every frame is a whole view. A tail longer than [`LOG_TAIL_CAP`] lines is
/// split over several frames, each repeating the same view and `seq`, so a
/// client that drops a view as not newer still reads the lines beside it. No
/// lines is one frame with an empty `log_json`.
fn state_frames(seq: u64, view: &PlayerView, tail: LogTail) -> Vec<Envelope> {
    frames_of(seq, view_json(view), tail)
}

/// [`state_frames`] for a view already written: a seat's or a spectator's.
fn frames_of(seq: u64, view_json: Vec<u8>, tail: LogTail) -> Vec<Envelope> {
    let LogTail {
        mut from,
        mut entries,
    } = tail;
    if entries.is_empty() {
        return vec![state_delta(seq, view_json, Vec::new())];
    }
    let mut out = Vec::with_capacity(entries.len().div_ceil(LOG_TAIL_CAP));
    while !entries.is_empty() {
        let rest = entries.split_off(entries.len().min(LOG_TAIL_CAP));
        let part = LogTail { from, entries };
        from = from.saturating_add(u32::try_from(part.entries.len()).unwrap_or(u32::MAX));
        let log_json = serde_json::to_vec(&part).unwrap_or_default();
        out.push(state_delta(seq, view_json.clone(), log_json));
        entries = rest;
    }
    out
}

/// A view as the wire carries it, written into a buffer sized for a busy
/// table up front: growing one from empty copied it a dozen times on the
/// way to a four-seat board's tens of kilobytes.
fn view_json(view: &PlayerView) -> Vec<u8> {
    let mut json = Vec::with_capacity(VIEW_JSON_HINT);
    if serde_json::to_writer(&mut json, view).is_err() {
        json.clear();
    }
    json
}

/// What a view's JSON is sized for before it is written: a busy four-seat
/// board is about 36 KB (`docs/perf-client.md`).
const VIEW_JSON_HINT: usize = 40 << 10;

fn state_delta(seq: u64, view_json: Vec<u8>, log_json: Vec<u8>) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
            game_id: String::new(),
            seq,
            view_json,
            log_json,
        })),
    }
}

fn choice_envelope(seq: u64, pending: &Pending) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::ChoiceRequest(v1::ChoiceRequest {
            game_id: String::new(),
            seq,
            pending_json: serde_json::to_vec(pending).unwrap_or_default(),
        })),
    }
}

#[cfg(test)]
pub(crate) mod tests;
