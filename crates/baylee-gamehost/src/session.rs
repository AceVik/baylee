//! One game session: an engine plus seats (humans and AI). Socket-free so
//! tests and both servers (engine-server dev harness, gateway) drive it
//! directly; transport lives with the callers.

use baylee_ai::{HeuristicAgent, pending_player};
use baylee_cards::dsl::CardDef;
use baylee_core::ids::{CardIndex, PlayerId};
use baylee_core::preset::{AIProfile, GamePreset, HouseRules, SeatController};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::state::CardLookup;
use baylee_protocol::v1::{self, Envelope};

/// Registry lookup backed by the compiled card pool.
pub struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static CardDef> {
        baylee_cards::by_index(index)
    }
}

/// What sits in a seat.
#[derive(Clone, Debug)]
pub enum SeatKind {
    /// A human connection (any number of these per game).
    Human,
    /// An auto-driven AI seat.
    Ai(HeuristicAgent),
}

/// A live game: engine plus the seat roster.
pub struct Session {
    engine: Engine<RegistryLookup>,
    seats: Vec<SeatKind>,
    seq: u64,
    /// Kept for the decision clock; the engine has its own copy for rules.
    house_rules: HouseRules,
}

impl Session {
    /// Starts a game from a preset; Open seats become humans, AI seats
    /// get a heuristic agent.
    #[must_use]
    pub fn new(preset: &GamePreset) -> Option<Self> {
        let seats: Vec<SeatKind> = preset
            .seats
            .iter()
            .map(|s| match &s.controller {
                SeatController::Ai(profile) => Some(SeatKind::Ai(HeuristicAgent::new(*profile))),
                _ => Some(SeatKind::Human),
            })
            .collect::<Option<_>>()?;
        let engine = Engine::new(preset, RegistryLookup).ok()?;
        Some(Self {
            engine,
            seats,
            seq: 0,
            house_rules: preset.house_rules.clone(),
        })
    }

    /// Human (non-AI) seats, as player ids in seat order.
    #[must_use]
    pub fn human_seats(&self) -> Vec<PlayerId> {
        self.seats
            .iter()
            .enumerate()
            .filter(|(_, k)| matches!(k, SeatKind::Human))
            .map(|(i, _)| PlayerId::new(i as u8))
            .collect()
    }

    /// Read-only state access (views).
    #[must_use]
    pub fn state(&self) -> &baylee_engine::state::GameState {
        self.engine.state()
    }

    /// The current pending choice.
    #[must_use]
    pub fn pending(&self) -> &Pending {
        self.engine.pending()
    }

    /// Drains AI-controlled pendings, then returns per-seat envelopes:
    /// a fresh hidden-information view for every human seat plus a
    /// choice request for the acting seat (or game over for everyone).
    /// Capped so an all-AI game can never hang the server.
    pub fn pump(&mut self) -> Vec<(PlayerId, Envelope)> {
        let mut out = Vec::new();
        for _ in 0..4096 {
            let pending = self.engine.pending().clone();
            let Some(player) = pending_player(&pending) else {
                if let Pending::GameOver(_) = &pending {
                    for seat in self.human_seats() {
                        out.push((seat, view_envelope(self.seq, &self.engine, seat, None)));
                        out.push((seat, choice_envelope(self.seq, &pending)));
                    }
                }
                return out;
            };
            let is_human = matches!(self.seats.get(player.get() as usize), Some(SeatKind::Human));
            if is_human {
                let priority = priority_holder(&pending);
                for seat in self.human_seats() {
                    out.push((seat, view_envelope(self.seq, &self.engine, seat, priority)));
                }
                out.push((player, choice_envelope(self.seq, &pending)));
                return out;
            }
            let action = match &self.seats[player.get() as usize] {
                // An AI seat is handed exactly what a networked client is
                // handed: its own filtered view, and the choice addressed
                // to it. Nothing else is reachable from here.
                SeatKind::Ai(agent) => {
                    let view = crate::view::player_view(
                        self.engine.state(),
                        player,
                        priority_holder(&pending),
                        self.seq,
                    );
                    agent.act(&view, &pending)
                }
                SeatKind::Human => unreachable!(),
            };
            if self.engine.apply(player, action).is_err() {
                // AI mis-evaluation: pass when possible, else give up.
                if matches!(pending, Pending::Priority { .. }) {
                    let _ = self.engine.apply(player, PlayerAction::PassPriority);
                } else {
                    return out;
                }
            }
            self.seq += 1;
        }
        out
    }

    /// How long a seat may sit on a decision, per the table's house rules
    /// (`0` = no limit).
    ///
    /// The clock itself belongs to the caller. Nothing below this line may
    /// read a wall clock: the rules kernel is deterministic, and a session
    /// that timed itself would replay differently on every machine.
    #[must_use]
    pub const fn decision_timeout_secs(&self) -> u32 {
        self.house_rules.decision_timeout_secs
    }

    /// The seat that currently owes an answer, if any.
    #[must_use]
    pub fn awaiting_seat(&self) -> Option<PlayerId> {
        pending_player(self.engine.pending())
    }

    /// The action to apply when a seat's decision clock runs out.
    ///
    /// The house agent answers rather than a hand-written table of defaults.
    /// It already produces a *legal* answer for every `Pending`, and a
    /// timeout that produced an illegal one would stall the very game it
    /// exists to unstick — the seat would be asked again, time out again,
    /// and the table would never move.
    #[must_use]
    pub fn timeout_action(&self) -> Option<(PlayerId, PlayerAction)> {
        let player = self.awaiting_seat()?;
        let agent = HeuristicAgent::new(AIProfile::default());
        let pending = self.engine.pending();
        let view = crate::view::player_view(
            self.engine.state(),
            player,
            priority_holder(pending),
            self.seq,
        );
        Some((player, agent.act(&view, pending)))
    }

    /// The sequence number a client should report back when it resumes.
    #[must_use]
    pub const fn seq(&self) -> u64 {
        self.seq
    }

    /// Everything a seat needs to render the game from scratch: its own
    /// view, plus the outstanding choice when this seat is the one being
    /// asked (or the game is over, which everyone is told about).
    ///
    /// Read-only on purpose. `pump` *advances* the game — it drives every AI
    /// seat until a human is needed — so rebuilding a client through it
    /// would let a reconnect take a turn on the AI's behalf.
    #[must_use]
    pub fn snapshot(&self, seat: PlayerId) -> Vec<Envelope> {
        let pending = self.engine.pending().clone();
        let mut out = vec![view_envelope(
            self.seq,
            &self.engine,
            seat,
            priority_holder(&pending),
        )];
        if pending_player(&pending) == Some(seat) || matches!(pending, Pending::GameOver(_)) {
            out.push(choice_envelope(self.seq, &pending));
        }
        out
    }

    /// Answers `ResumeGame{last_seq}`: the snapshot when the client is
    /// behind, nothing when it is already current.
    ///
    /// Returning nothing matters — a client that reconnects without having
    /// missed anything should not be made to re-render the whole table.
    #[must_use]
    pub fn resume(&self, seat: PlayerId, last_seq: u64) -> Vec<Envelope> {
        if last_seq >= self.seq {
            return Vec::new();
        }
        self.snapshot(seat)
    }

    /// Applies a human action, then pumps the AI until a human is needed.
    ///
    /// # Errors
    /// When the action isn't the acting seat's legal answer.
    pub fn act(
        &mut self,
        player: PlayerId,
        action: PlayerAction,
    ) -> Result<Vec<(PlayerId, Envelope)>, String> {
        if !matches!(self.seats.get(player.get() as usize), Some(SeatKind::Human)) {
            return Err("not a human seat".to_string());
        }
        if self.engine.apply(player, action).is_err() {
            return Err("illegal action for your seat".to_string());
        }
        self.seq += 1;
        Ok(self.pump())
    }
}

/// The seat holding priority, if the game is currently offering it.
///
/// Priority is not stored on the state; it only exists as the pending choice,
/// so a view has to be told about it rather than deriving it.
pub(crate) fn priority_holder(pending: &Pending) -> Option<PlayerId> {
    match pending {
        Pending::Priority { player, .. } => Some(*player),
        _ => None,
    }
}

fn view_envelope(
    seq: u64,
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    priority: Option<PlayerId>,
) -> Envelope {
    let view = crate::view::player_view(engine.state(), seat, priority, seq);
    Envelope {
        msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
            game_id: String::new(),
            seq,
            view_json: serde_json::to_vec(&view).unwrap_or_default(),
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
mod tests {
    use super::*;
    use baylee_core::ids::PrintRef;
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, HouseRules, PrintInfo, SeatSpec,
    };

    fn island() -> CardIndex {
        baylee_cards::by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
            .unwrap()
            .index
    }

    fn test_preset() -> GamePreset {
        let deck: Vec<DeckEntry> = (0..60)
            .map(|_| DeckEntry {
                card: island(),
                print: PrintRef::new(0),
            })
            .collect();
        let mk = |ai: bool| SeatSpec {
            controller: if ai {
                SeatController::Ai(AIProfile::default())
            } else {
                SeatController::Open
            },
            deck: deck.clone(),
            sideboard: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        GamePreset {
            format: FormatId::Freeform,
            seed: 7,
            dev_mode: false,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            seats: vec![mk(false), mk(true)],
        }
    }

    /// A session starts, the human answers mulligans, and the AI seat is
    /// driven automatically between human choices.
    #[test]
    fn session_pumps_ai_between_human_choices() {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let human = session.human_seats()[0];
        let mut human_choices = 0;
        for _ in 0..50 {
            let envelopes = session.pump();
            if envelopes.is_empty() {
                break;
            }
            for (_, env) in envelopes {
                let Some(v1::envelope::Msg::ChoiceRequest(req)) = env.msg else {
                    continue;
                };
                let pending: Pending = serde_json::from_slice(&req.pending_json).unwrap();
                human_choices += 1;
                let action = match pending {
                    Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                    Pending::ChooseAttackers { .. } => {
                        PlayerAction::DeclareAttackers { attackers: vec![] }
                    }
                    _ => PlayerAction::PassPriority,
                };
                let _ = session.act(human, action);
            }
        }
        assert!(human_choices > 0, "the human received choices");
    }

    /// Plays a couple of steps so the session has a sequence number to be
    /// behind or current on.
    fn started_session() -> (Session, PlayerId) {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let human = session.human_seats()[0];
        let _ = session.pump();
        let _ = session.act(human, PlayerAction::MulliganKeep);
        assert!(session.seq() > 0, "the game moved");
        (session, human)
    }

    /// A client that has applied everything is not made to re-render the
    /// whole table just because it reconnected.
    #[test]
    fn a_current_client_gets_nothing_back() {
        let (session, human) = started_session();
        assert!(session.resume(human, session.seq()).is_empty());
    }

    /// A client that missed everything gets its seat rebuilt — and asking for
    /// it does not move the game. Rebuilding through `pump` would have played
    /// the AI seats forward as a side effect of someone reconnecting.
    #[test]
    fn a_stale_client_is_rebuilt_without_advancing_the_game() {
        let (session, human) = started_session();
        let pending_before = format!("{:?}", session.pending());
        let seq_before = session.seq();

        let envelopes = session.resume(human, 0);
        assert!(
            envelopes
                .iter()
                .any(|e| matches!(e.msg, Some(v1::envelope::Msg::StateDelta(_)))),
            "the seat's own view is part of the rebuild"
        );
        assert_eq!(session.seq(), seq_before, "resume moved the sequence");
        assert_eq!(
            format!("{:?}", session.pending()),
            pending_before,
            "resume moved the game"
        );
    }

    /// The snapshot carries the outstanding choice only for the seat that
    /// owes an answer: a spectating reconnect must not be handed someone
    /// else's decision.
    #[test]
    fn only_the_asked_seat_is_sent_the_choice() {
        let (session, human) = started_session();
        let others: Vec<PlayerId> = (0..2).map(PlayerId::new).filter(|p| *p != human).collect();
        let asked = super::pending_player(session.pending());
        for seat in others {
            let has_choice = session
                .snapshot(seat)
                .iter()
                .any(|e| matches!(e.msg, Some(v1::envelope::Msg::ChoiceRequest(_))));
            assert_eq!(
                has_choice,
                asked == Some(seat) || matches!(session.pending(), Pending::GameOver(_)),
                "choice went to the wrong seat"
            );
        }
    }

    /// The clock is the table's, not the engine's.
    #[test]
    fn the_decision_clock_comes_from_the_house_rules() {
        let mut preset = test_preset();
        preset.house_rules.decision_timeout_secs = 42;
        let session = Session::new(&preset).expect("session builds");
        assert_eq!(session.decision_timeout_secs(), 42);
    }

    /// A seat that runs out of time is answered legally, and the game moves.
    ///
    /// This is the property that matters: an *illegal* timeout answer would
    /// leave the same seat being asked the same question forever, which is
    /// the exact failure the clock exists to prevent.
    #[test]
    fn a_timed_out_seat_is_answered_legally() {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let _ = session.pump();
        let (player, action) = session
            .timeout_action()
            .expect("somebody is being asked something");
        assert_eq!(Some(player), session.awaiting_seat());

        let seq_before = session.seq();
        session
            .act(player, action)
            .expect("the timeout answer is legal");
        assert!(session.seq() > seq_before, "the game moved on");
    }

    /// Answering by timeout over and over drives the game forward rather than
    /// deadlocking on a pending nobody can satisfy.
    #[test]
    fn repeated_timeouts_keep_the_game_moving() {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let _ = session.pump();
        let mut answered = 0;
        for _ in 0..40 {
            let Some((player, action)) = session.timeout_action() else {
                break;
            };
            if session.act(player, action).is_err() {
                break;
            }
            answered += 1;
        }
        assert!(answered > 5, "only {answered} timeouts were answered");
    }
}
