//! One game session: an engine plus seats (humans and AI). Socket-free so
//! tests and both servers (engine-server dev harness, gateway) drive it
//! directly; transport lives with the callers.

use baylee_ai::{HeuristicAgent, pending_player};
use baylee_cards::dsl::CardDef;
use baylee_core::ids::{CardIndex, PlayerId};
use baylee_core::preset::{AIProfile, GamePreset, HouseRules, PrintInfo, SeatController};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::state::CardLookup;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, SeatIdentity};

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
}

/// A live game: engine plus the seat roster.
pub struct Session {
    engine: Engine<RegistryLookup>,
    seats: Vec<SeatKind>,
    seq: u64,
    /// How many times the *question* has changed — see
    /// [`Session::decision_seq`].
    decisions: u64,
    /// Kept for the decision clock; the engine has its own copy for rules.
    house_rules: HouseRules,
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
    /// What clients call this game (see [`Session::describe`]).
    game_id: String,
    /// What clients call each seat (see [`Session::describe`]).
    names: Vec<String>,
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
                SeatController::Ai(profile) => Some(SeatKind::Ai(
                    HeuristicAgent::new(*profile)
                        .with_teams(preset.seats.iter().map(|s| s.team).collect()),
                )),
                _ => Some(SeatKind::Human),
            })
            .collect::<Option<_>>()?;
        let engine = Engine::new(preset, RegistryLookup).ok()?;
        Some(Self {
            engine,
            seats,
            seq: 0,
            decisions: 0,
            house_rules: preset.house_rules.clone(),
            prints: preset.prints.clone(),
            revealed: preset
                .seats
                .iter()
                .map(|spec| own_prints(spec, preset.prints.len()))
                .collect(),
            teams: preset.seats.iter().map(|s| s.team).collect(),
            game_id: String::new(),
            names: Vec::new(),
        })
    }

    /// The seats that are played over a socket, in seat order.
    ///
    /// Humans, and any AI seat whose controls have been taken — the two are
    /// the same thing to everything that sends views, which is why the name
    /// is about the socket and not about who is on the other end of it.
    #[must_use]
    pub fn human_seats(&self) -> Vec<PlayerId> {
        self.seats
            .iter()
            .enumerate()
            .filter(|(_, k)| k.answers_over_socket())
            .map(|(i, _)| PlayerId::new(i as u8))
            .collect()
    }

    /// What kind of chair a seat is, or `None` when it is not a seat here.
    ///
    /// The three answers are not interchangeable to a caller deciding whether
    /// someone may sit down: an AI chair can be taken over, a human chair can
    /// be joined, and a chair already being driven must be refused — and
    /// [`Session::take_over`] alone cannot tell the last two apart, because
    /// both are already answering over a socket.
    #[must_use]
    pub fn seat_kind(&self, seat: PlayerId) -> Option<&SeatKind> {
        self.seats.get(seat.get() as usize)
    }

    /// Takes the controls of an AI seat, so it answers over a socket instead.
    ///
    /// Returns whether it took: a seat that is already human, already driven,
    /// or is not a seat at all is refused rather than silently accepted,
    /// because a caller that thinks it is driving a chair it is not would sit
    /// waiting for a question the house AI has already answered.
    ///
    /// The game is not disturbed. Whatever the agent has already played
    /// stands, and the next question addressed to this seat simply goes out
    /// over the wire rather than into [`HeuristicAgent::act`].
    pub fn take_over(&mut self, seat: PlayerId) -> bool {
        let Some(kind) = self.seats.get_mut(seat.get() as usize) else {
            return false;
        };
        let SeatKind::Ai(agent) = kind else {
            return false;
        };
        *kind = SeatKind::Driven(agent.clone());
        true
    }

    /// Hands a driven seat back to the house AI it was taken from.
    ///
    /// Returns whether it was being driven. The agent comes back with it —
    /// it was kept rather than dropped precisely so that a developer who
    /// disconnects mid-game leaves a playable opponent behind instead of a
    /// table that stops at the next question nobody is there to answer.
    pub fn release(&mut self, seat: PlayerId) -> bool {
        let Some(kind) = self.seats.get_mut(seat.get() as usize) else {
            return false;
        };
        let SeatKind::Driven(agent) = kind else {
            return false;
        };
        *kind = SeatKind::Ai(agent.clone());
        true
    }

    /// The seats that won: one for a solo winner, every seat on the team for
    /// a team win — the dead ones included, because a team wins as a team
    /// (CR 104.2b) — and none at all for a draw.
    ///
    /// It lives here rather than in the engine because a `Victor::Team` names
    /// a team and a client's roster names seats, and the seat roster is what
    /// this session already keeps.
    #[must_use]
    pub fn winning_seats(&self, result: baylee_engine::win::GameResult) -> Vec<PlayerId> {
        let Some(victor) = result.winner else {
            return Vec::new();
        };
        (0..self.seats.len())
            .map(|i| PlayerId::new(i as u8))
            .filter(|seat| {
                victor.includes(
                    *seat,
                    self.teams.get(seat.get() as usize).copied().flatten(),
                )
            })
            .collect()
    }

    /// Names the table for the payload clients are sent.
    ///
    /// The rules kernel has never heard of an account, so a host supplies this
    /// once, after building the session and before the first socket. A seat
    /// nobody names falls back to its number rather than to an empty chair.
    pub fn describe(&mut self, game_id: String, names: Vec<String>) {
        self.game_id = game_id;
        self.names = names;
    }

    /// The once-per-game payload for a seat: the seat roster, the print table
    /// as far as this seat has earned it, and the view schema version.
    #[must_use]
    pub fn game_static(&self, seat: PlayerId) -> GameStatic {
        let seats = self
            .seats
            .iter()
            .enumerate()
            .map(|(i, kind)| SeatIdentity {
                player: PlayerId::new(i as u8),
                display_name: self
                    .names
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("Seat {i}")),
                is_ai: kind.is_ai_chair(),
                team: self.teams.get(i).copied().flatten(),
            })
            .collect();
        let shown = self
            .revealed
            .get(seat.get() as usize)
            .map_or(&[][..], Vec::as_slice);
        crate::view::game_static(self.game_id.clone(), seat, seats, &self.prints, shown)
    }

    /// [`Session::game_static`] as the envelope a socket sends.
    ///
    /// Every host — the in-process one included — takes this payload off the
    /// wire rather than building it from a preset it happens to have in hand.
    /// A field that only the local path filled in would be missing in exactly
    /// the case nobody tests at a desk.
    #[must_use]
    pub fn game_static_envelope(&self, seat: PlayerId) -> Envelope {
        let statics = self.game_static(seat);
        Envelope {
            msg: Some(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
                game_id: self.game_id.clone(),
                view_version: baylee_view::VIEW_VERSION,
                static_json: serde_json::to_vec(&statics).unwrap_or_default(),
            })),
        }
    }

    /// Marks every printing a view showed a seat; true when any was new.
    fn reveal(&mut self, seat: PlayerId, view: &baylee_view::PlayerView) -> bool {
        let Some(shown) = self.revealed.get_mut(seat.get() as usize) else {
            return false;
        };
        let mut grew = false;
        for print in view.prints() {
            if let Some(slot) = shown.get_mut(print.get() as usize)
                && !*slot
            {
                *slot = true;
                grew = true;
            }
        }
        grew
    }

    /// A seat's view, preceded by a fresh opening payload when this view is
    /// the first to show it one of the game's printings.
    ///
    /// The order matters: the entry has to be there before the object that
    /// points at it, or the client draws a card it cannot key an image on.
    fn view_envelopes(&mut self, seat: PlayerId, priority: Option<PlayerId>) -> Vec<Envelope> {
        let view = crate::view::player_view(
            self.engine.state(),
            seat,
            priority,
            self.seq,
            Some(self.engine.pending()),
            self.engine.automation(seat).hold.suppresses(),
        );
        let mut out = Vec::new();
        if self.reveal(seat, &view) {
            out.push(self.game_static_envelope(seat));
        }
        out.push(view_envelope(self.seq, &view));
        out
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
                        let envelopes = self.view_envelopes(seat, None);
                        out.extend(envelopes.into_iter().map(|env| (seat, env)));
                        out.push((seat, choice_envelope(self.seq, &pending)));
                    }
                }
                return out;
            };
            let is_human = self
                .seats
                .get(player.get() as usize)
                .is_some_and(SeatKind::answers_over_socket);
            if is_human {
                let priority = priority_holder(&pending);
                for seat in self.human_seats() {
                    let envelopes = self.view_envelopes(seat, priority);
                    out.extend(envelopes.into_iter().map(|env| (seat, env)));
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
                        Some(&pending),
                        self.engine.automation(player).hold.suppresses(),
                    );
                    agent.act(&view, &pending)
                }
                // Both answer over a socket, so `pump` returned above.
                SeatKind::Human | SeatKind::Driven(_) => unreachable!(),
            };
            let mut moves_the_game = !action.is_automation_setting();
            if self.engine.apply(player, action).is_err() {
                // AI mis-evaluation: pass when possible, else give up.
                if matches!(pending, Pending::Priority { .. }) {
                    let _ = self.engine.apply(player, PlayerAction::PassPriority);
                    moves_the_game = true;
                } else {
                    return out;
                }
            }
            self.seq += 1;
            if moves_the_game {
                self.decisions += 1;
            }
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
            Some(pending),
            self.engine.automation(player).hold.suppresses(),
        );
        Some((player, agent.act(&view, pending)))
    }

    /// The sequence number a client should report back when it resumes.
    #[must_use]
    pub const fn seq(&self) -> u64 {
        self.seq
    }

    /// How many times the question has changed — the anchor for a decision
    /// clock, and deliberately *not* [`Session::seq`].
    ///
    /// `seq` counts frames, and a frame is produced by anything a seat says,
    /// including the two things a seat may say while it is not the one being
    /// asked: a priority hold and a standing answer. A clock anchored to `seq`
    /// therefore restarts when the *opponent* presses `F6`, or reconnects
    /// (an attach replays every remembered answer), which hands out unlimited
    /// thinking time to whoever spams either. This counts only actions that
    /// moved the game, so an automation setting leaves the clock exactly where
    /// it was.
    #[must_use]
    pub const fn decision_seq(&self) -> u64 {
        self.decisions
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
        // Read-only, so no printing is revealed here: this rebuilds a state a
        // `pump` already showed this seat, and the reveal happened there.
        let view = crate::view::player_view(
            self.engine.state(),
            seat,
            priority_holder(&pending),
            self.seq,
            Some(&pending),
            self.engine.automation(seat).hold.suppresses(),
        );
        let mut out = vec![view_envelope(self.seq, &view)];
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
        if !self
            .seats
            .get(player.get() as usize)
            .is_some_and(SeatKind::answers_over_socket)
        {
            return Err("not a human seat".to_string());
        }
        // Read before the action is spent: an automation setting is the one
        // thing the engine takes from a seat that is not being asked, and it
        // leaves the question standing. See [`Session::decision_seq`].
        let moves_the_game = !action.is_automation_setting();
        if self.engine.apply(player, action).is_err() {
            return Err("illegal action for your seat".to_string());
        }
        self.seq += 1;
        if moves_the_game {
            self.decisions += 1;
        }
        Ok(self.pump())
    }
}

/// The printings a seat already knows because they are its own.
///
/// A player has seen their own decklist; nothing is revealed by handing it
/// back. Everything outside this set has to be earned by seeing a card.
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

fn view_envelope(seq: u64, view: &baylee_view::PlayerView) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::StateDelta(v1::StateDelta {
            game_id: String::new(),
            seq,
            view_json: serde_json::to_vec(view).unwrap_or_default(),
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
            capabilities: baylee_core::preset::SeatCapabilities::default(),
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

    /// The whole point of a driven seat: the question stops going to the
    /// house AI and starts going out over the wire.
    ///
    /// Seat 1 is an AI chair. Left alone it answers its own mulligan inside
    /// `pump`, and the game never stops for it — `pump` only returns when a
    /// seat that answers over a socket is on the clock. Taken over, the same
    /// question comes back addressed to seat 1, which is what somebody at the
    /// other end of the harness answers.
    #[test]
    fn a_driven_seat_is_asked_instead_of_answering_itself() {
        let human = PlayerId::new(0);
        let ai = PlayerId::new(1);

        // Left alone, nothing ever stops for seat 1.
        let mut session = Session::new(&test_preset()).expect("the preset builds");
        let _ = session.pump();
        let _ = session.act(human, PlayerAction::MulliganKeep);
        assert_ne!(
            session.awaiting_seat(),
            Some(ai),
            "the house AI answers for itself"
        );

        // Taken over, the same question is addressed to it.
        let mut session = Session::new(&test_preset()).expect("the preset builds");
        assert!(session.take_over(ai), "seat 1 is an AI chair");
        let _ = session.pump();
        let _ = session.act(human, PlayerAction::MulliganKeep);
        assert_eq!(
            session.awaiting_seat(),
            Some(ai),
            "a driven seat is asked over the wire"
        );
    }

    /// Releasing hands the chair back mid-game, so a developer who
    /// disconnects leaves a playable opponent rather than a table stopped at
    /// a question nobody is there to answer.
    #[test]
    fn releasing_a_seat_gives_it_back_to_the_house_ai() {
        let human = PlayerId::new(0);
        let ai = PlayerId::new(1);
        let mut session = Session::new(&test_preset()).expect("the preset builds");
        assert!(session.take_over(ai));
        let _ = session.pump();
        let _ = session.act(human, PlayerAction::MulliganKeep);
        assert_eq!(session.awaiting_seat(), Some(ai), "stopped for the driver");

        assert!(session.release(ai), "it was being driven");
        // The question it was stopped on is now answered by the agent it was
        // taken from, so the table moves again without the driver.
        let _ = session.pump();
        assert_ne!(
            session.awaiting_seat(),
            Some(ai),
            "the house AI has the chair back"
        );
    }

    /// Both refusals, because a caller that believed it was driving a chair
    /// it was not would sit waiting for a question the house AI has already
    /// answered.
    #[test]
    fn only_an_ai_chair_can_be_taken_over_and_only_once() {
        let mut session = Session::new(&test_preset()).expect("the preset builds");
        assert!(
            !session.take_over(PlayerId::new(0)),
            "seat 0 is a human seat"
        );
        assert!(
            !session.take_over(PlayerId::new(7)),
            "there is no seat 7 to take"
        );
        assert!(!session.release(PlayerId::new(1)), "it is not driven yet");
        assert!(session.take_over(PlayerId::new(1)));
        assert!(
            !session.take_over(PlayerId::new(1)),
            "and it cannot be taken twice"
        );
    }

    /// A chair does not change what it *is* when somebody takes the
    /// controls, so the roster a client draws stays put.
    #[test]
    fn a_driven_chair_is_still_an_ai_chair_on_the_roster() {
        let mut session = Session::new(&test_preset()).expect("the preset builds");
        session.describe("g".to_string(), vec!["You".into(), "House AI".into()]);
        let before = session.game_static(PlayerId::new(0));
        assert!(session.take_over(PlayerId::new(1)));
        let after = session.game_static(PlayerId::new(0));
        assert_eq!(
            before.seats.iter().map(|s| s.is_ai).collect::<Vec<_>>(),
            after.seats.iter().map(|s| s.is_ai).collect::<Vec<_>>(),
            "the roster says what the chair is, not who is holding it"
        );
    }

    fn forest() -> CardIndex {
        baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("Forest is in the registry")
            .index
    }

    /// Two seats with nothing in common: seat 0 plays Islands, seat 1 plays
    /// Forests and starts one on the battlefield, so it is visible at once.
    fn split_preset() -> GamePreset {
        let deck = |card: CardIndex, print: u16| -> Vec<DeckEntry> {
            (0..60)
                .map(|_| DeckEntry {
                    card,
                    print: PrintRef::new(print),
                })
                .collect()
        };
        let mk = |card, print, battlefield: Vec<DeckEntry>| SeatSpec {
            controller: SeatController::Open,
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: deck(card, print),
            sideboard: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: battlefield,
            emblems: vec![],
            team: None,
        };
        let print = |n: u128| PrintInfo {
            scryfall_id: uuid::Uuid::from_u128(n),
            lang: "EN".into(),
            finish: Finish::Normal,
        };
        GamePreset {
            format: FormatId::Freeform,
            seed: 7,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![print(1), print(2)],
            seats: vec![
                mk(island(), 0, vec![]),
                mk(
                    forest(),
                    1,
                    vec![DeckEntry {
                        card: forest(),
                        print: PrintRef::new(1),
                    }],
                ),
            ],
        }
    }

    /// The print table is the union of every deck at the table, so handing a
    /// seat all of it would hand it the opponent's decklist — the one piece of
    /// hidden information with no game object to hide behind.
    #[test]
    fn a_seat_is_not_handed_the_other_decks_printings() {
        let session = Session::new(&split_preset()).expect("session");

        let mine = session.game_static(PlayerId::new(0));
        assert!(mine.print(PrintRef::new(0)).is_some(), "its own deck");
        assert!(
            mine.print(PrintRef::new(1)).is_none(),
            "seat 0 has not seen a Forest, and must not learn that one exists"
        );

        let theirs = session.game_static(PlayerId::new(1));
        assert!(theirs.print(PrintRef::new(1)).is_some());
        assert!(theirs.print(PrintRef::new(0)).is_none());
        assert_eq!(
            mine.prints.len(),
            theirs.prints.len(),
            "a hole, not a shorter table: the index is the PrintRef"
        );
    }

    /// Seeing the card earns the printing, and the entry arrives before the
    /// view that points at it.
    #[test]
    fn a_printing_is_earned_by_seeing_the_card() {
        let mut session = Session::new(&split_preset()).expect("session");
        let routed = session.pump();

        let addressed = |seat: PlayerId| -> Vec<&Envelope> {
            routed
                .iter()
                .filter(|(p, _)| *p == seat)
                .map(|(_, env)| env)
                .collect()
        };
        let is_static = |env: &&Envelope| matches!(env.msg, Some(v1::envelope::Msg::GameStatic(_)));
        let is_view = |env: &&Envelope| matches!(env.msg, Some(v1::envelope::Msg::StateDelta(_)));

        let seat0 = addressed(PlayerId::new(0));
        let statics = seat0.iter().position(is_static);
        let view = seat0.iter().position(is_view);
        assert!(
            statics.is_some(),
            "seat 0 was shown a Forest it had never been shown before"
        );
        assert!(
            statics < view,
            "the print entry has to arrive before the object that points at it"
        );
        assert!(
            session
                .game_static(PlayerId::new(0))
                .print(PrintRef::new(1))
                .is_some(),
            "and it stays earned"
        );

        assert_eq!(
            addressed(PlayerId::new(1))
                .iter()
                .filter(|env| is_static(env))
                .count(),
            0,
            "nothing new was shown to the seat that owns the card"
        );
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

    /// The view a seat is sent, decoded back out of its envelope.
    fn seat_view(session: &Session, seat: PlayerId) -> baylee_view::PlayerView {
        session
            .snapshot(seat)
            .into_iter()
            .find_map(|e| match e.msg {
                Some(v1::envelope::Msg::StateDelta(delta)) => {
                    Some(serde_json::from_slice(&delta.view_json).expect("the view decodes"))
                }
                _ => None,
            })
            .expect("every snapshot carries a view")
    }

    /// The same, out of what an action actually *sent* the seat.
    ///
    /// The difference matters for anything a client has to be told: `snapshot`
    /// is read-only and builds a fresh view every time it is asked, so it
    /// would report a field as delivered even if the live path — `act`, then
    /// `pump` — never produced a frame at all.
    fn routed_view(routed: &[(PlayerId, Envelope)], seat: PlayerId) -> baylee_view::PlayerView {
        routed
            .iter()
            .filter(|(s, _)| *s == seat)
            .find_map(|(_, e)| match &e.msg {
                Some(v1::envelope::Msg::StateDelta(delta)) => {
                    Some(serde_json::from_slice(&delta.view_json).expect("the view decodes"))
                }
                _ => None,
            })
            .expect("a pump sends every human seat its own view")
    }

    /// A priority hold is a statement about what its owner intends to respond
    /// to, which makes it exactly the kind of read a player is entitled to
    /// keep. It reaches that seat's own view and nobody else's.
    ///
    /// It also has to reach the view *at all*: the hold lives in the engine's
    /// `SeatAutomation` and not in the `GameState` the view is built from, so
    /// there is a parameter carrying it across and nothing but a test says it
    /// was filled in.
    #[test]
    fn a_seat_sees_its_own_hold_and_not_the_other_seats() {
        // Both seats human, so both are sent a view and the second half of
        // this test has something to read.
        let mut preset = test_preset();
        preset.seats[1].controller = SeatController::Open;
        let mut session = Session::new(&preset).expect("session builds");
        let seat = PlayerId::new(0);
        let turn = seat_view(&session, seat).turn;
        assert!(!seat_view(&session, seat).priority_held, "nothing held yet");

        // Set while the opening mulligan is pending, which is the point: the
        // engine takes an automation setting from any seated player whether or
        // not it is that player's decision, and without that a hold could
        // never be cancelled.
        let routed = session
            .act(
                seat,
                PlayerAction::SetPriorityHold(
                    baylee_engine::choice::PriorityHold::UntilEndOfTurn { turn },
                ),
            )
            .expect("a seat may state a standing order at any time");

        assert!(
            routed_view(&routed, seat).priority_held,
            "the seat that set the hold was never sent a view saying so"
        );
        assert!(
            !routed_view(&routed, PlayerId::new(1)).priority_held,
            "one seat's standing order is not the other's to read"
        );
    }

    /// The one hold that answers rather than withholds does not light the
    /// indicator, because it never keeps a decision from being offered.
    #[test]
    fn passing_when_there_is_nothing_to_do_is_not_a_hold() {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let seat = PlayerId::new(0);
        session
            .act(
                seat,
                PlayerAction::SetPriorityHold(
                    baylee_engine::choice::PriorityHold::PassWhenNothingToDo,
                ),
            )
            .expect("a seat may state a standing order at any time");
        assert!(
            !seat_view(&session, seat).priority_held,
            "it fires only where passing was the sole legal action, so a seat \
             running it is never actually being kept from a decision"
        );
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
    /// The roster a client is sent: who is at the table, which of them is the
    /// house, and the print table without which a `PrintRef` names no card.
    #[test]
    fn the_opening_payload_describes_the_table() {
        let mut session = Session::new(&test_preset()).expect("session");
        session.describe("g1".to_string(), vec!["Ada".into(), "House AI".into()]);
        let statics = session.game_static(PlayerId::new(0));

        assert_eq!(statics.view_version, baylee_view::VIEW_VERSION);
        assert_eq!(statics.game_id, "g1");
        assert_eq!(statics.your_seat, PlayerId::new(0));
        assert_eq!(statics.seat_name(PlayerId::new(0)), "Ada");
        assert_eq!(statics.seat_name(PlayerId::new(1)), "House AI");
        assert!(!statics.seats[0].is_ai);
        assert!(statics.seats[1].is_ai, "seat 1 of the fixture is the house");
        assert_eq!(statics.prints.len(), 1);
    }

    /// A seat nobody named still has to be nameable, or the client draws a
    /// board with an empty chair opposite.
    #[test]
    fn an_unnamed_seat_falls_back_to_its_number() {
        let session = Session::new(&test_preset()).expect("session");
        let statics = session.game_static(PlayerId::new(0));
        assert_eq!(statics.seat_name(PlayerId::new(0)), "Seat 0");
        assert_eq!(statics.seat_name(PlayerId::new(1)), "Seat 1");
    }

    /// The version rides outside the payload so a client can refuse a table it
    /// cannot render without first decoding the very structure that changed.
    #[test]
    fn the_opening_envelope_states_the_view_version_in_the_open() {
        let mut session = Session::new(&test_preset()).expect("session");
        session.describe("g1".to_string(), vec!["Ada".into()]);
        let envelope = session.game_static_envelope(PlayerId::new(0));
        let Some(v1::envelope::Msg::GameStatic(msg)) = envelope.msg else {
            panic!("the opening payload is a GameStatic envelope");
        };
        assert_eq!(msg.view_version, baylee_view::VIEW_VERSION);
        assert_eq!(msg.game_id, "g1");
        let decoded: GameStatic =
            serde_json::from_slice(&msg.static_json).expect("the payload decodes");
        assert_eq!(decoded.your_seat, PlayerId::new(0));
    }

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
