//! One game session: an engine plus seats (humans and AI). Socket-free so
//! tests and both servers (engine-server dev harness, gateway) drive it
//! directly; transport lives with the callers.

use baylee_ai::{HeuristicAgent, policy_seed};
use baylee_cards::dsl::CardDef;
use baylee_core::ids::{CardIndex, PlayerId, SeatSet};
use baylee_core::preset::{AIProfile, GamePreset, HouseRules, PrintInfo, SeatController};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::state::CardLookup;
use baylee_engine::zone::ZoneLocation;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{
    ClockAnswer, GameStatic, HouseAnswer, LOG_TAIL_CAP, LogEvent, LogTail, PlayerView, SeatIdentity,
};

use crate::log::GameLog;

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
}

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
            roster_dirty: vec![false; preset.seats.len()],
            game_id: String::new(),
            names: Vec::new(),
            log,
            told: vec![0; preset.seats.len()],
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
        self.roster_changed();
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
        self.roster_changed();
        true
    }

    /// The house sits down at a player's chair, because nobody is answering
    /// on it.
    ///
    /// Returns whether it took: only a human chair can be stood in for. An AI
    /// chair has nobody to wait for, and a driven one is the dev harness's,
    /// which hands itself back when its socket drops.
    ///
    /// *When* to call this is not a question this crate can answer — it needs
    /// a wall clock, and nothing below the transport may read one. The caller
    /// owns the deadline; `HouseRules::reconnect_window_secs` is how long the
    /// table said to wait.
    pub fn stand_in(&mut self, seat: PlayerId) -> bool {
        let teams = self.teams.clone();
        let Some(kind) = self.seats.get_mut(seat.get() as usize) else {
            return false;
        };
        if !matches!(kind, SeatKind::Human) {
            return false;
        }
        // The same agent an AI chair at this table would get, teams included
        // — a stand-in that did not know who its allies were would play the
        // seat's own team as an enemy.
        *kind = SeatKind::StandIn(HeuristicAgent::new(AIProfile::default()).with_teams(teams));
        self.roster_changed();
        self.log.note(LogEvent::StandIn { player: seat });
        true
    }

    /// The player came back; the chair is theirs again.
    ///
    /// Returns whether the house was holding it. The agent is dropped rather
    /// than kept — unlike [`Session::release`], there is nothing to hand back
    /// *to*: the chair was never an AI chair, and a fresh stand-in is built
    /// if the player leaves again.
    pub fn hand_back(&mut self, seat: PlayerId) -> bool {
        let Some(kind) = self.seats.get_mut(seat.get() as usize) else {
            return false;
        };
        if !matches!(kind, SeatKind::StandIn(_)) {
            return false;
        }
        *kind = SeatKind::Human;
        self.roster_changed();
        self.log.note(LogEvent::Returned { player: seat });
        true
    }

    /// A chair changed hands, so every seat's roster is out of date.
    ///
    /// The roster travels in [`GameStatic`], which is sent once when a socket
    /// attaches — so before this existed, a chair could change hands and the
    /// table was simply never told. Every seat is marked, not just the one
    /// that changed: the roster lists all of them.
    fn roster_changed(&mut self) {
        self.roster_dirty.iter_mut().for_each(|d| *d = true);
    }

    /// The seats that won: one for a solo winner, every seat on the team for
    /// a team win — the dead ones included, because a team wins as a team
    /// (CR 104.2c) — and none at all for a draw.
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
        // The public game identifier arrives exactly here, once, before the
        // first socket — so this is where an AI chair stops playing with the
        // no-identifier derivation and starts playing with its own (#87).
        // Every chair that holds an agent is reseeded, including the two that
        // are not currently answering: a seat handed back by `release` or
        // `hand_back` plays on with the agent it kept.
        for (i, kind) in self.seats.iter_mut().enumerate() {
            let seed = policy_seed(&game_id, seat_byte(i));
            match kind {
                SeatKind::Ai(agent) | SeatKind::Driven(agent) | SeatKind::StandIn(agent) => {
                    *agent = agent.clone().with_seed(seed);
                }
                SeatKind::Human => {}
            }
        }
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
                away: kind.is_away(),
                team: self.teams.get(i).copied().flatten(),
            })
            .collect();
        let shown = self
            .revealed
            .get(seat.get() as usize)
            .map_or(&[][..], Vec::as_slice);
        crate::view::game_static(
            self.game_id.clone(),
            seat,
            seats,
            &self.prints,
            shown,
            &self.house_rules,
        )
    }

    /// [`Session::game_static`] as the envelope a socket sends.
    ///
    /// Every host — the in-process one included — takes this payload off the
    /// wire rather than building it from a preset it happens to have in hand.
    /// A field that only the local path filled in would be missing in exactly
    /// the case nobody tests at a desk.
    ///
    /// Producing the envelope *is* the act of telling the seat, which is why
    /// this takes `&mut self` where [`Session::game_static`] does not: it
    /// clears the seat's pending roster change. Anything that only wants to
    /// look reads `game_static`.
    #[must_use]
    pub fn game_static_envelope(&mut self, seat: PlayerId) -> Envelope {
        if let Some(dirty) = self.roster_dirty.get_mut(seat.get() as usize) {
            *dirty = false;
        }
        let statics = self.game_static(seat);
        Envelope {
            msg: Some(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
                game_id: self.game_id.clone(),
                view_version: baylee_view::VIEW_VERSION,
                static_json: serde_json::to_vec(&statics).unwrap_or_default(),
            })),
        }
    }

    /// Marks every printing a seat was shown, by its view or its log; true
    /// when any was new.
    fn reveal(
        &mut self,
        seat: PlayerId,
        prints: impl IntoIterator<Item = baylee_core::ids::PrintRef>,
    ) -> bool {
        let Some(shown) = self.revealed.get_mut(seat.get() as usize) else {
            return false;
        };
        let mut grew = false;
        for print in prints {
            if let Some(slot) = shown.get_mut(print.get() as usize)
                && !*slot
            {
                *slot = true;
                grew = true;
            }
        }
        grew
    }

    /// A seat's view and the log lines it has not been sent, preceded by a
    /// fresh opening payload when these are the first to show it one of the
    /// game's printings, or when a chair has changed hands since this seat
    /// was last told the roster.
    ///
    /// The order matters: the entry has to be there before the object or the
    /// line that points at it, or the client draws a card it cannot key an
    /// image on.
    fn view_envelopes(&mut self, seat: PlayerId) -> Vec<Envelope> {
        let awaiting = crate::view::awaiting_for(&self.engine, seat);
        let view = crate::view::player_view(
            self.engine.state(),
            seat,
            self.seq,
            self.engine.pending_for(seat),
            &crate::view::SeatContext {
                awaiting,
                deciding: self.deciding(),
                held: self.engine.automation(seat).hold.suppresses(),
                owed: crate::view::owed_payment(&self.engine),
                decision_remaining_ms: awaiting.and_then(|s| self.decision_remaining_ms(s)),
            },
            &self.house_answered,
        );
        let mut out = Vec::new();
        let tail = self.log_tail(seat);
        // Two separate `let`s: `reveal` marks printings as shown, so folding
        // it into an `||` would let the other half short-circuit it away.
        let revealed = self.reveal(seat, view.prints().chain(tail.prints()));
        let roster_moved = self
            .roster_dirty
            .get(seat.get() as usize)
            .copied()
            .unwrap_or(false);
        if revealed || roster_moved {
            out.push(self.game_static_envelope(seat));
        }
        out.extend(state_frames(self.seq, &view, tail));
        out
    }

    /// The log lines `seat` has not been sent yet, as it may know them, and
    /// marks them sent.
    ///
    /// Nothing for a seat nobody answers over a socket. An agent is handed a
    /// view and a question, never a log: a log is a history, and what a
    /// house seat may know of the game is exactly what its view shows now.
    /// A chair the house holds for an absent player has nobody to tell, and
    /// the player is sent the whole log when they are back
    /// ([`Session::retell_log`]).
    fn log_tail(&mut self, seat: PlayerId) -> LogTail {
        let at = seat.get() as usize;
        let socket = self
            .seats
            .get(at)
            .is_some_and(SeatKind::answers_over_socket);
        let lines = self.log.len();
        let Some(told) = self.told.get_mut(at).filter(|_| socket) else {
            return LogTail::default();
        };
        let from = (*told).min(lines);
        *told = lines;
        self.log.seal(lines);
        LogTail {
            from: u32::try_from(from).unwrap_or(u32::MAX),
            entries: self.log.told(seat, from, lines),
        }
    }

    /// Every log line `seat` has been sent, from the first, for a rebuild.
    ///
    /// Read-only: nothing is marked sent and no printing is earned, because
    /// both happened when the lines were first sent. Lines not sent yet are
    /// not here either; they come with the next view, as they would have.
    fn log_sent(&self, seat: PlayerId) -> LogTail {
        let at = seat.get() as usize;
        if !self
            .seats
            .get(at)
            .is_some_and(SeatKind::answers_over_socket)
        {
            return LogTail::default();
        }
        let sent = self.told.get(at).copied().unwrap_or(0).min(self.log.len());
        LogTail {
            from: 0,
            entries: self.log.told(seat, 0, sent),
        }
    }

    /// `seat`'s next view carries its whole log again, from the first line.
    ///
    /// For a socket that has just attached and will be pumped: it holds none
    /// of the log, and whatever went to the seat while nobody was on it was
    /// dropped on the way. A resync that is sent a [`Session::snapshot`]
    /// needs no call, since the snapshot carries every line already sent.
    pub fn retell_log(&mut self, seat: PlayerId) {
        if let Some(told) = self.told.get_mut(seat.get() as usize) {
            *told = 0;
        }
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
    /// choice request for every seat being asked (or game over for
    /// everyone). Capped so an all-AI game can never hang the server.
    pub fn pump(&mut self) -> Vec<(PlayerId, Envelope)> {
        let mut out = Vec::new();
        for _ in 0..4096 {
            let awaited = self.engine.awaited();
            if awaited.is_empty() {
                let pending = self.engine.pending().clone();
                if let Pending::GameOver(_) = &pending {
                    for seat in self.human_seats() {
                        let envelopes = self.view_envelopes(seat);
                        out.extend(envelopes.into_iter().map(|env| (seat, env)));
                        out.push((seat, choice_envelope(self.seq, &pending)));
                    }
                }
                return out;
            }
            // The house answers first, in seat order: during the opening
            // mulligans several seats are asked at once, and a human who is
            // still deciding must not hold up an AI chair's keep.
            let house = awaited.iter().find_map(|seat| {
                let socket = self
                    .seats
                    .get(seat.get() as usize)
                    .is_some_and(SeatKind::answers_over_socket);
                let pending = self.engine.pending_for(seat).filter(|_| !socket)?;
                Some((seat, pending.clone()))
            });
            let Some((player, pending)) = house else {
                for seat in self.human_seats() {
                    let envelopes = self.view_envelopes(seat);
                    out.extend(envelopes.into_iter().map(|env| (seat, env)));
                }
                for seat in awaited.iter() {
                    if let Some(pending) = self.engine.pending_for(seat) {
                        out.push((seat, choice_envelope(self.seq, pending)));
                    }
                }
                return out;
            };
            let action = match &self.seats[player.get() as usize] {
                // Both receive a filtered view. Scouting checks the live
                // seat kind separately and denies a human's stand-in.
                SeatKind::Ai(agent) | SeatKind::StandIn(agent) => {
                    let view = crate::view::player_view(
                        self.engine.state(),
                        player,
                        self.seq,
                        Some(&pending),
                        &crate::view::SeatContext {
                            awaiting: crate::view::awaiting_for(&self.engine, player),
                            deciding: self.deciding(),
                            held: self.engine.automation(player).hold.suppresses(),
                            owed: crate::view::owed_payment(&self.engine),
                            // No clock in a view an agent answers from. This
                            // number is made of elapsed wall time, and
                            // machine speed is not an authorized input to a
                            // decision: an agent that read it would play the
                            // same position differently on a slow machine,
                            // legally and invisibly. Same invariant as #87.
                            decision_remaining_ms: None,
                        },
                        &self.house_answered,
                    );
                    let context = self.engine.decision_context();
                    let scouting = agent.scouting_request(&pending).and_then(|request| {
                        crate::scouting::request(
                            &self.seats,
                            &self.scouting_decks,
                            self.engine.state(),
                            player,
                            request,
                        )
                    });
                    scouting.as_ref().map_or_else(
                        || agent.act_with_context(&view, &pending, &context),
                        |report| agent.act_with_scouting(&view, &pending, &context, report),
                    )
                }
                // Both answer over a socket, so `pump` returned above.
                SeatKind::Human | SeatKind::Driven(_) => unreachable!(),
            };
            let by = self.seats[player.get() as usize]
                .is_away()
                .then_some(HouseAnswer::StandIn);
            let deciding = self.deciding();
            let moves_the_game = self.apply_house_action(player, action);
            self.seq += 1;
            if moves_the_game {
                self.moved(player, deciding);
                self.house_answered[player.get() as usize] = by;
            }
        }
        out
    }

    /// An invalid agent proposal must not leave an untimed seat stalled (#180).
    fn apply_house_action(&mut self, player: PlayerId, action: PlayerAction) -> bool {
        let moves = !action.is_automation_setting();
        let deciding = self.deciding();
        let asked = self.answering(player, &action);
        if self.engine.apply(player, action).is_ok() {
            self.log_answer(player, &asked, deciding, None);
            return moves;
        }
        // Re-read the actual question: a rejected proposal can have entered
        // a casting/payment wizard. The ordinary timeout policy covers all
        // question kinds instead of a positive list containing only Priority.
        let fallback = self
            .house_action(player)
            .expect("refused AI action left no decision");
        let asked = self.answering(player, &fallback);
        self.engine.apply(player, fallback).expect(
            "both AI proposal and recovery were refused; refusing to silently stall the table",
        );
        self.log_answer(player, &asked, deciding, None);
        true
    }

    /// What the log needs to know of `player`'s answer before it is spent.
    fn answering(&self, player: PlayerId, action: &PlayerAction) -> Asked {
        Asked {
            hand: self
                .engine
                .state()
                .zones
                .list(ZoneLocation::Hand(player))
                .len(),
            took: matches!(action, PlayerAction::MulliganTake),
            bottomed: match action {
                PlayerAction::ChooseObjects { objects } => objects.len(),
                _ => 0,
            },
        }
    }

    /// Writes into the log what `player`'s answer did: what a clock answered
    /// in their place, a mulligan or a keep, and then everything the journal
    /// recorded while it was applied.
    ///
    /// `deciding` is [`Session::deciding`] before the answer. A seat that
    /// leaves it by answering, and has not left the game, has kept: its hand
    /// as it answered, less what the answer put on the bottom.
    fn log_answer(
        &mut self,
        player: PlayerId,
        asked: &Asked,
        deciding: SeatSet,
        clock: Option<ClockAnswer>,
    ) {
        if let Some(answer) = clock {
            self.log.note(LogEvent::TimedOut { player, answer });
        }
        if asked.took {
            self.log.note(LogEvent::Mulliganed { player });
        } else if deciding.contains(player)
            && !self.deciding().contains(player)
            && !self.engine.state().has_left(player)
        {
            let cards = asked.hand.saturating_sub(asked.bottomed);
            self.log.note(LogEvent::Kept {
                player,
                cards: u8::try_from(cards).unwrap_or(u8::MAX),
            });
        }
        self.log.consume(self.engine.state());
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

    /// How long a seat may be gone before the house takes its chair, per the
    /// table's house rules (`0` = wait forever).
    ///
    /// A second clock rather than the same one, because it answers a
    /// different question. A seat with no socket is on no decision clock at
    /// all — nobody should lose on time to a question they never saw — and
    /// that is exactly the state this one measures. It is also independent of
    /// `decision_timeout_secs`: a table that gives its players all the time in
    /// the world still must not sit forever waiting on a closed laptop.
    #[must_use]
    pub const fn reconnect_window_secs(&self) -> u32 {
        self.house_rules.reconnect_window_secs
    }

    /// Tell this session how much of `seat`'s decision clock is left, as read
    /// by whoever owns the clock.
    ///
    /// `seq` is the [`Session::asked_at`] the reading was taken against, and
    /// passing it is not ceremony: the caller reads its clock *before*
    /// handing a frame in, and handling that frame may move the game, so by
    /// the time a view is built the number can already belong to a question
    /// nobody is being asked any more.
    ///
    /// `remaining_ms` of `None` states that **no decision clock is running** —
    /// an untimed table, an AI chair, or a seat waiting out its reconnect
    /// window rather than deciding. That is a different statement from never
    /// having called this at all, and [`Session::decision_remaining_ms`]
    /// treats them differently.
    pub fn set_decision_remaining(&mut self, seat: PlayerId, seq: u64, remaining_ms: Option<u32>) {
        if let Some(slot) = self.clock.get_mut(seat.get() as usize) {
            *slot = Some((seq, remaining_ms));
        }
    }

    /// How long `seat` has left to answer, in milliseconds, or `None` when it
    /// is not being asked or no decision clock is running for it.
    ///
    /// Almost all of this is the anchor check. A reading is only true of the
    /// question it was taken for, so once the seat has been asked a new one
    /// ([`Session::asked_at`]) the reading is discarded and the seat is given the table's
    /// whole allowance instead — which is exactly right, because a question
    /// that has only just been asked has had no time taken off it. Without
    /// that branch the first view of every new question would carry the
    /// *previous* question's leftovers, and a seat would be shown four
    /// seconds to answer something it was asked a moment ago.
    ///
    /// The allowance is refused for a chair the clock does not run for. An AI
    /// chair is on no clock, and a chair the house is holding is waiting on a
    /// player rather than deciding, so neither inherits an allowance merely
    /// because the seat asked before them had one.
    #[must_use]
    pub fn decision_remaining_ms(&self, seat: PlayerId) -> Option<u32> {
        let asked_at = self.asked_at(seat)?;
        if let Some(Some((seq, reading))) = self.clock.get(seat.get() as usize)
            && *seq == asked_at
        {
            return *reading;
        }
        if !self
            .seat_kind(seat)
            .is_some_and(SeatKind::answers_over_socket)
        {
            return None;
        }
        match self.house_rules.decision_timeout_secs {
            0 => None,
            secs => Some(secs.saturating_mul(1_000)),
        }
    }

    /// Every seat that owes an answer: all that are still deciding their
    /// opening mulligans, and after them the one the game is waiting on.
    #[must_use]
    pub fn awaited(&self) -> SeatSet {
        self.engine.awaited()
    }

    /// The seat the table is waiting on, as every view names it: the one
    /// [`Engine::pending`](baylee_engine::engine::Engine::pending) is
    /// addressed to. During the opening mulligans that is the lowest seat
    /// still deciding, and [`Session::awaited`] is every one of them.
    #[must_use]
    pub fn awaiting_seat(&self) -> Option<PlayerId> {
        self.engine.pending().asked()
    }

    /// The [`Session::decision_seq`] at which `seat` was asked the question
    /// it owes now, or `None` when it owes none: what a decision clock for
    /// `seat` is anchored to.
    ///
    /// Not `decision_seq` itself, because during the opening mulligans
    /// several seats are asked at once and each answers in its own time. One
    /// seat's keep moves the game without asking any other seat anything
    /// new, and a clock anchored to the shared count would restart every
    /// other seat's deadline at every keep: free time for whoever waits.
    /// Everywhere else one seat is asked, and its anchor moves exactly when
    /// the shared count does ([`Session::moved`]).
    #[must_use]
    pub fn asked_at(&self, seat: PlayerId) -> Option<u64> {
        if !self.engine.awaited().contains(seat) {
            return None;
        }
        self.asked_at.get(seat.get() as usize).copied()
    }

    /// The seats owing an opening mulligan question: the same set every
    /// view carries as `PlayerView::deciding`.
    fn deciding(&self) -> SeatSet {
        crate::view::deciding(&self.engine)
    }

    /// The game moved, by `answered`'s answer or concession: counts the
    /// question and re-anchors every seat that is now asked something new.
    ///
    /// `deciding` is [`Session::deciding`] read before the move. A seat
    /// keeps its anchor only if it owed an opening mulligan question before,
    /// still owes one and did not answer: that seat is asked exactly what it
    /// was asked, and its clock runs on. Every other seat still being asked
    /// is asked anew, and outside the mulligans that is every move, as it
    /// was when the one count was the anchor: a bystander's concession or
    /// draw offer changes the awaited seat's question without that seat
    /// saying anything.
    fn moved(&mut self, answered: PlayerId, deciding: SeatSet) {
        self.decisions += 1;
        let still = self.deciding();
        for seat in self.engine.awaited().iter() {
            let holds = seat != answered && deciding.contains(seat) && still.contains(seat);
            if !holds && let Some(at) = self.asked_at.get_mut(seat.get() as usize) {
                *at = self.decisions;
            }
        }
    }

    /// The action to apply when a seat's decision clock runs out: the
    /// question's answer that does nothing
    /// ([`baylee_engine::choice::timeout_answer`]), else the house's
    /// ([`Self::house_action`]) (#258).
    ///
    /// A seat that ran out of time passes, attacks and blocks with nothing,
    /// keeps its hand and declines what declining leaves alone. It used to
    /// be played by the house in full, which cast its spells and sent its
    /// creatures in on its behalf. The client writes the countdown into the
    /// button this answer is, and it can only do that for an answer it can
    /// know. A question with no answer that does nothing (a discard, a
    /// target) is still the house's.
    #[must_use]
    pub fn timeout_action(&self, seat: PlayerId) -> Option<PlayerAction> {
        let pending = self.engine.pending_for(seat)?;
        baylee_engine::choice::timeout_answer(pending).or_else(|| self.house_action(seat))
    }

    /// What the house answers for `seat`, if it is being asked anything.
    ///
    /// The house agent answers rather than a hand-written table of defaults.
    /// It already produces a *legal* answer for every `Pending`, and a
    /// timeout that produced an illegal one would stall the very game it
    /// exists to unstick — the seat would be asked again, time out again,
    /// and the table would never move.
    #[must_use]
    pub fn house_action(&self, player: PlayerId) -> Option<PlayerAction> {
        let pending = self.engine.pending_for(player)?;
        // Teams included, as `stand_in` builds it: without them every other
        // chair reads as an enemy, the seat's own partner among them.
        let agent = HeuristicAgent::new(AIProfile::default()).with_teams(self.teams.clone());
        let view = crate::view::player_view(
            self.engine.state(),
            player,
            self.seq,
            Some(pending),
            &crate::view::SeatContext {
                awaiting: crate::view::awaiting_for(&self.engine, player),
                deciding: self.deciding(),
                held: self.engine.automation(player).hold.suppresses(),
                owed: crate::view::owed_payment(&self.engine),
                // As in `pump`, and pointedly so here: this view exists
                // because a clock ran out, so it is the one place the
                // expired number could reach a rules decision.
                decision_remaining_ms: None,
            },
            &self.house_answered,
        );
        Some(agent.act(&view, pending))
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
    /// view, every log line it has been sent, plus the outstanding choice
    /// when this seat is the one being asked (or the game is over, which
    /// everyone is told about).
    ///
    /// Read-only on purpose. `pump` *advances* the game — it drives every AI
    /// seat until a human is needed — so rebuilding a client through it
    /// would let a reconnect take a turn on the AI's behalf.
    ///
    /// The log is here because a rebuild is asked for when frames were lost
    /// (a lagging socket, a `ResumeGame`), and the lines in them are gone
    /// with the views. A client appends by `LogTail::from`, so the lines it
    /// already holds cost bytes and nothing else.
    #[must_use]
    pub fn snapshot(&self, seat: PlayerId) -> Vec<Envelope> {
        self.rebuild(seat, self.log_sent(seat))
    }

    /// The seat's view and the question it owes, and no log: what a refused
    /// answer is handed back.
    ///
    /// A refusal loses no frame, so the seat holds every line it was sent,
    /// and a snapshot would send it the whole log again for each one — a
    /// few bytes of action answered with every line of the game.
    #[must_use]
    pub fn reask(&self, seat: PlayerId) -> Vec<Envelope> {
        self.rebuild(seat, LogTail::default())
    }

    fn rebuild(&self, seat: PlayerId, log: LogTail) -> Vec<Envelope> {
        let own = self.engine.pending_for(seat);
        let awaiting = crate::view::awaiting_for(&self.engine, seat);
        // Read-only, so no printing is revealed here: this rebuilds a state a
        // `pump` already showed this seat, and the reveal happened there.
        let view = crate::view::player_view(
            self.engine.state(),
            seat,
            self.seq,
            own,
            &crate::view::SeatContext {
                awaiting,
                deciding: self.deciding(),
                held: self.engine.automation(seat).hold.suppresses(),
                owed: crate::view::owed_payment(&self.engine),
                decision_remaining_ms: awaiting.and_then(|s| self.decision_remaining_ms(s)),
            },
            &self.house_answered,
        );
        let mut out = state_frames(self.seq, &view, log);
        let over = Some(self.engine.pending()).filter(|p| matches!(p, Pending::GameOver(_)));
        if let Some(pending) = own.or(over) {
            out.push(choice_envelope(self.seq, pending));
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
        self.answer(player, action, None)
    }

    /// The decision clock ran out on `seat`: the house answers this one
    /// question for it, with the answer that does nothing where there is one
    /// ([`Self::timeout_action`], [`by_clock`]), and the seat is marked as
    /// answered by the clock until it answers one itself.
    ///
    /// `None` when `seat` is not the one being asked, which is how a timer
    /// that fired after the question moved on is told apart from one that
    /// is still due: one seat's expired clock must never take another seat's
    /// decision. `Some(Err)` when the engine refused the house's answer,
    /// which a caller treats like any refused answer.
    ///
    /// Its own door rather than [`Session::timeout_action`] followed by
    /// [`Session::act`], because through `act` the session cannot tell the
    /// clock's answer from the player's — and the views `act` builds on the
    /// way out are the ones that have to say which it was.
    pub fn answer_by_clock(
        &mut self,
        seat: PlayerId,
        asked_at: u64,
    ) -> Option<Result<Vec<(PlayerId, Envelope)>, String>> {
        if self.asked_at(seat)? != asked_at {
            return None;
        }
        let pending = self.engine.pending_for(seat)?.clone();
        by_clock(
            self,
            &pending,
            |session, action| session.answer(seat, action, Some(HouseAnswer::Clock)),
            |session| session.house_action(seat),
        )
    }

    /// Applies an answer for a socket seat and records who produced it.
    fn answer(
        &mut self,
        player: PlayerId,
        action: PlayerAction,
        by: Option<HouseAnswer>,
    ) -> Result<Vec<(PlayerId, Envelope)>, String> {
        let Some(kind) = self.seats.get(player.get() as usize) else {
            return Err("not a human seat".to_string());
        };
        if !kind.answers_over_socket() {
            return Err("not a human seat".to_string());
        }
        let clock = matches!(by, Some(HouseAnswer::Clock))
            .then(|| clock_answer(self.engine.pending_for(player), &action));
        let by = by.filter(|_| !kind.is_ai_chair());
        // Read before the action is spent: an automation setting is the one
        // thing the engine takes from a seat that is not being asked, and it
        // leaves the question standing. See [`Session::decision_seq`].
        let moves_the_game = !action.is_automation_setting();
        let deciding = self.deciding();
        let asked = self.answering(player, &action);
        if self.engine.apply(player, action).is_err() {
            return Err("illegal action for your seat".to_string());
        }
        self.log_answer(player, &asked, deciding, clock);
        self.seq += 1;
        if moves_the_game {
            self.moved(player, deciding);
            self.house_answered[player.get() as usize] = by;
        }
        Ok(self.pump())
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
/// The refusal is not reachable with today's pool: nothing makes an empty
/// declaration of attackers or blockers illegal. It will be once a creature
/// that attacks each combat if able (CR 508.1d) or a lure (CR 509.1c) is
/// read, and a clock that stopped at the refusal would ask the same seat
/// the same question forever. So the order lives here, apart from the
/// session, where a refusing engine can be written as a closure.
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
    let view_json = serde_json::to_vec(view).unwrap_or_default();
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
mod tests {
    use super::*;

    /// The awaited seat's clock answer, as a one-seat table asks it.
    fn timeout(session: &Session) -> Option<(PlayerId, PlayerAction)> {
        let seat = session.awaiting_seat()?;
        Some((seat, session.timeout_action(seat)?))
    }

    /// The house's answer for the awaited seat, as a one-seat table asks it.
    fn house(session: &Session) -> Option<(PlayerId, PlayerAction)> {
        let seat = session.awaiting_seat()?;
        Some((seat, session.house_action(seat)?))
    }

    /// `seat`'s clock running out on the question it owes now.
    fn clock_answers(
        session: &mut Session,
        seat: PlayerId,
    ) -> Option<Result<Vec<(PlayerId, Envelope)>, String>> {
        let asked_at = session.asked_at(seat)?;
        session.answer_by_clock(seat, asked_at)
    }
    use baylee_core::ids::PrintRef;
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, HouseRules, PrintInfo, SeatSpec,
    };
    use baylee_view::LogEntry;

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
            commanders: vec![],
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

    fn agent() -> HeuristicAgent {
        HeuristicAgent::new(AIProfile::default())
    }

    /// A distinct number per seat kind, and the guard the table below needs:
    /// all three predicates are `matches!`, so a fifth kind answers `false`
    /// to every one of them and is silently a seat nobody sends anything to.
    /// This match is exhaustive.
    fn kind_index(kind: &SeatKind) -> usize {
        match kind {
            SeatKind::Human => 0,
            SeatKind::Ai(_) => 1,
            SeatKind::Driven(_) => 2,
            SeatKind::StandIn(_) => 3,
        }
    }

    /// **A chair and whoever is answering for it are two different
    /// questions, and there are three of them.** Each seat kind is a
    /// different combination, which is the whole reason all four exist —
    /// reading "does it answer over the wire" as "is it a human" in even one
    /// place leaves a driven seat being played by the house AI it was taken
    /// from, and reading "is it an AI chair" as "is the house answering"
    /// renames a player's chair in the lobby thirty seconds after their
    /// laptop shut.
    ///
    /// The four rows are four distinct answers, which is asserted rather
    /// than left to be read: two kinds agreeing on all three would be two
    /// names for one seat.
    #[test]
    fn a_seat_is_read_three_ways_and_no_two_kinds_answer_alike() {
        let table = [
            // (kind, answers over a socket, is an AI chair, is away)
            (SeatKind::Human, true, false, false),
            (SeatKind::Ai(agent()), false, true, false),
            (SeatKind::Driven(agent()), true, true, false),
            (SeatKind::StandIn(agent()), false, false, true),
        ];
        let mut answers = Vec::new();
        for (kind, socket, ai_chair, away) in &table {
            assert_eq!(
                (
                    kind.answers_over_socket(),
                    kind.is_ai_chair(),
                    kind.is_away()
                ),
                (*socket, *ai_chair, *away),
                "seat kind {}",
                kind_index(kind)
            );
            answers.push((*socket, *ai_chair, *away));
        }
        assert_eq!(
            table
                .iter()
                .map(|(k, ..)| kind_index(k))
                .collect::<Vec<_>>(),
            (0..4).collect::<Vec<_>>(),
            "the table is every kind exactly once, in declaration order"
        );
        answers.sort_unstable();
        answers.dedup();
        assert_eq!(answers.len(), 4, "two kinds are one seat under two names");
    }

    #[test]
    fn issue_180_a_refused_non_priority_action_recovers() {
        let mut session = Session::new(&test_preset()).unwrap();
        let player = session.awaiting_seat().expect("opening mulligan");
        assert!(matches!(session.engine.pending(), Pending::Mulligan { .. }));
        let before = session.engine.snapshot_hash();
        assert!(session.apply_house_action(
            player,
            PlayerAction::PlayLand {
                card: baylee_core::ids::ObjectId::new(999_999, 0),
            }
        ));
        assert_ne!(session.engine.snapshot_hash(), before);
    }

    fn teamed_preset(teams: [Option<u8>; 4]) -> GamePreset {
        let mut preset = test_preset();
        let seat = preset.seats[0].clone();
        preset.seats = teams
            .iter()
            .map(|team| SeatSpec {
                team: *team,
                ..seat.clone()
            })
            .collect();
        preset
    }

    /// A team wins as a team, the dead included (CR 104.2c), and the roster
    /// is where that is turned back into seats: the engine names a
    /// `Victor::Team` and a client's roster names chairs.
    ///
    /// The fourth seat is on no team at all, which is the row that says the
    /// answer is read off each seat rather than off the winner: a seat with
    /// `None` is on no winning team however the game ended.
    #[test]
    fn a_team_win_names_every_chair_on_the_team() {
        use baylee_engine::win::{EndReason, GameResult, Victor};
        let session = Session::new(&teamed_preset([Some(1), Some(2), Some(1), None]))
            .expect("a four-seat game");
        let seats = |winner| {
            session.winning_seats(GameResult {
                winner,
                reason: EndReason::LastTeamStanding,
            })
        };

        assert_eq!(
            seats(Some(Victor::Team(1))),
            vec![PlayerId::new(0), PlayerId::new(2)],
            "both chairs on the team, in seat order"
        );
        assert_eq!(seats(Some(Victor::Team(2))), vec![PlayerId::new(1)]);
        assert!(
            seats(Some(Victor::Team(3))).is_empty(),
            "a team nobody at this table plays for"
        );
        assert_eq!(
            seats(Some(Victor::Player(PlayerId::new(3)))),
            vec![PlayerId::new(3)],
            "a seat that won as itself, and it is on no team"
        );
        assert!(seats(None).is_empty(), "a draw has no winning seat to name");
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

    /// The bug this whole mechanism exists for: seat 0 walks away holding the
    /// decision, and the table waits on it forever.
    ///
    /// A seat with no socket is on no decision clock — deliberately, because
    /// nobody should lose on time to a question they never saw — so nothing
    /// else was ever going to move this game again.
    #[test]
    fn a_chair_the_house_stands_in_for_stops_holding_up_the_table() {
        let gone = PlayerId::new(0);
        let mut session = Session::new(&test_preset()).expect("the preset builds");
        let _ = session.pump();
        assert_eq!(session.awaiting_seat(), Some(gone), "seat 0 owes an answer");
        let stuck = session.decision_seq();
        assert_eq!(
            session
                .pump()
                .iter()
                .filter(|(_, e)| matches!(e.msg, Some(v1::envelope::Msg::ChoiceRequest(_))))
                .count(),
            1,
            "pumping again just asks seat 0 the same question"
        );
        assert_eq!(session.decision_seq(), stuck, "and the game has not moved");

        assert!(session.stand_in(gone), "seat 0 is a player's chair");
        let _ = session.pump();
        assert!(
            session.decision_seq() > stuck,
            "the house answered and the table moved on"
        );
    }

    /// Both refusals in one place, because `stand_in` is reached from a
    /// deadline rather than from a request: a caller that stood in for the
    /// wrong chair would have no one to tell.
    #[test]
    fn only_a_players_chair_can_be_stood_in_for() {
        let mut session = Session::new(&test_preset()).expect("the preset builds");
        assert!(
            !session.stand_in(PlayerId::new(1)),
            "seat 1 is an AI chair; it is not waiting for anyone"
        );
        assert!(
            !session.stand_in(PlayerId::new(7)),
            "there is no seat 7 to stand in for"
        );
        assert!(
            !session.hand_back(PlayerId::new(0)),
            "seat 0 is already the player's"
        );
        assert!(session.take_over(PlayerId::new(1)), "now it is driven");
        assert!(
            !session.stand_in(PlayerId::new(1)),
            "a driven chair hands itself back when its socket drops"
        );
        assert!(session.stand_in(PlayerId::new(0)));
        assert!(
            !session.stand_in(PlayerId::new(0)),
            "and the house cannot sit down twice"
        );
    }

    /// The chair was only ever borrowed.
    #[test]
    fn a_player_who_comes_back_gets_their_chair_back() {
        let gone = PlayerId::new(0);
        let mut session = Session::new(&test_preset()).expect("the preset builds");
        let _ = session.pump();
        assert!(session.stand_in(gone));
        assert!(session.hand_back(gone), "the house was holding it");
        let _ = session.pump();
        assert_eq!(
            session.awaiting_seat(),
            Some(gone),
            "the question is theirs again"
        );
    }

    /// A held chair still belongs to the player who left it.
    ///
    /// Two fields rather than one, because a seat that renamed itself to the
    /// house AI after a thirty-second hiccup would be telling the table
    /// something untrue — and would go on saying it after the player was
    /// back, since a roster is sent once.
    #[test]
    fn a_held_chair_says_away_rather_than_calling_itself_an_ai() {
        let mut session = Session::new(&test_preset()).expect("the preset builds");
        session.describe("g".to_string(), vec!["You".into(), "House AI".into()]);
        assert!(session.stand_in(PlayerId::new(0)));
        let seen = session.game_static(PlayerId::new(1));
        assert!(seen.seats[0].away, "seat 0 is being stood in for");
        assert!(!seen.seats[0].is_ai, "but it is still a player's chair");
        assert!(session.hand_back(PlayerId::new(0)));
        assert!(
            !session.game_static(PlayerId::new(1)).seats[0].away,
            "and it stops saying so"
        );
    }

    /// Whether a `GameStatic` was addressed to a seat in one pump.
    fn told_the_roster(out: &[(PlayerId, Envelope)], seat: PlayerId) -> bool {
        out.iter()
            .any(|(p, e)| *p == seat && matches!(e.msg, Some(v1::envelope::Msg::GameStatic(_))))
    }

    /// The roster travels in `GameStatic`, which is sent once when a socket
    /// attaches — so a chair could change hands and the table was simply
    /// never told. That had been true of `take_over`/`release` since they
    /// existed; standing in is what made it visible.
    #[test]
    fn a_seat_learns_the_roster_changed_when_a_chair_changes_hands() {
        let watcher = PlayerId::new(0);
        let mut session = Session::new(&test_preset()).expect("the preset builds");
        session.describe("g".to_string(), vec!["You".into(), "House AI".into()]);
        let _ = session.pump();
        assert!(
            !told_the_roster(&session.pump(), watcher),
            "nothing changed, so nothing is re-sent"
        );
        assert!(session.take_over(PlayerId::new(1)));
        assert!(
            told_the_roster(&session.pump(), watcher),
            "the chair changed hands and the table is told"
        );
        assert!(
            !told_the_roster(&session.pump(), watcher),
            "told once, not on every view after"
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
            commanders: vec![],
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

    /// The view names the seat the table is waiting for, which is a wider
    /// question than who holds priority — and the field answered the narrow
    /// one until `VIEW_VERSION` 23.
    ///
    /// Priority (CR 117) exists only while the engine is offering it, so a
    /// seat taking a mulligan, picking blockers or discarding to hand size
    /// holds none and was reported as nobody. Its three readers — the stack
    /// head's "waiting for", the seat caret and the board model's pod — all
    /// mean "waiting on them", so all three went blank on every question that
    /// was not a priority pass. `pending_player` is the question they were
    /// asking.
    ///
    /// The opponent's copy is the half that cannot be worked out client-side:
    /// a session sends the pending question only to the seat it is addressed
    /// to, so the other seat has nothing else to read it off.
    ///
    /// The opening mulligans have no such other seat: every seat is asked
    /// its own at once (#257), so each seat's view names itself until it has
    /// kept, and nobody after.
    #[test]
    fn a_seat_that_holds_no_priority_is_still_the_seat_being_waited_for() {
        // Both seats human, so the seat that is not being asked is sent a
        // view and the assertions on it have something to read.
        let mut preset = test_preset();
        preset.seats[1].controller = SeatController::Open;
        let mut session = Session::new(&preset).expect("session builds");
        let (zero, one) = (PlayerId::new(0), PlayerId::new(1));

        assert!(
            matches!(session.pending(), Pending::Mulligan { .. }),
            "the game opens on a question nobody holds priority for"
        );
        for seat in [zero, one] {
            assert_eq!(
                seat_view(&session, seat).awaiting,
                Some(seat),
                "a seat deciding its mulligan is told the table is waiting \
                 for it"
            );
        }
        session
            .act(zero, PlayerAction::MulliganKeep)
            .expect("a keep");
        assert_eq!(
            seat_view(&session, zero).awaiting,
            None,
            "a seat that has kept is asked nothing"
        );
        assert_eq!(seat_view(&session, one).awaiting, Some(one));
        session
            .act(one, PlayerAction::MulliganKeep)
            .expect("a keep");

        // Play the game out with the house agent answering both chairs, and
        // hold both views against the seat that actually owes an answer.
        // Bounded on the questions seen rather than on the loop, because a
        // run that stopped early would assert almost nothing.
        let agent = HeuristicAgent::new(AIProfile::default());
        let mut priority_questions = 0usize;
        let mut other_questions = 0usize;
        for _ in 0..600 {
            let Some(seat) = session.awaiting_seat() else {
                break;
            };
            if matches!(session.pending(), Pending::Priority { .. }) {
                priority_questions += 1;
            } else {
                other_questions += 1;
            }
            for viewer in [zero, one] {
                assert_eq!(
                    seat_view(&session, viewer).awaiting,
                    Some(seat),
                    "every question names the seat that owes the answer, in \
                     both views"
                );
            }
            let view = seat_view(&session, seat);
            let action = agent.act(&view, session.pending());
            if session.act(seat, action).is_err() {
                break;
            }
        }
        assert!(
            priority_questions > 10,
            "the game reached priority repeatedly: {priority_questions}"
        );
        assert!(
            other_questions > 1,
            "and asked questions that were not priority: {other_questions}"
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

    /// A table where seat 0 can cast Swords to Plowshares at seat 1's Roaming
    /// Throne, which prints ward {2}, and has two Plains left over to pay it.
    fn a_table_with_a_warded_creature() -> GamePreset {
        let named = |name: &str| DeckEntry {
            card: baylee_cards::decks::by_name(name).expect("a card this pool compiles"),
            print: PrintRef::new(0),
        };
        let mut preset = test_preset();
        preset.seats[1].controller = SeatController::Open;
        // Three Plains: one pays for the spell, two answer the tax. A seat
        // that could not pay at all would be inside a different question.
        preset.seats[0].starting_hand = Some(vec![named("Swords to Plowshares")]);
        preset.seats[0].starting_battlefield = vec![named("Plains"); 3];
        preset.seats[1].starting_hand = Some(vec![]);
        preset.seats[1].starting_battlefield = vec![named("Roaming Throne")];
        preset
    }

    /// Plays the table into ward's CR 605.3a window: cast the removal at the
    /// warded creature and agree to pay. Answers whether the tax was ever
    /// offered, so a fixture that stopped somewhere else cannot pass quietly.
    fn drive_into_ward_s_payment_window(session: &mut Session, payer: PlayerId) -> bool {
        // Cast the removal at the warded creature and agree to pay the tax,
        // taking every answer from the engine's own offer. The tap before the
        // cast is not scene-setting: `can_cast` probes the *pool* and not the
        // untapped lands (`casting::affordable`), so a spell is castable only
        // once its mana is floating — which is the same rule the payment
        // window exists to serve.
        let mut agreed = false;
        let mut cast = false;
        for _ in 0..200 {
            if session.engine.payment_window().is_some() {
                break;
            }
            let Some(seat) = session.awaiting_seat() else {
                break;
            };
            let action = match session.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                // The Throne's own "as this enters, choose a creature type",
                // which seat 1 answers before anything else can happen.
                Pending::ChooseSubtype { options, .. } => PlayerAction::ChooseSubtype(options[0]),
                Pending::ChooseTargets { options, .. } => PlayerAction::ChooseTargets {
                    objects: options.first().copied().into_iter().collect(),
                    players: vec![],
                },
                Pending::YesNo { .. } => {
                    // The resolution is already suspended with the price
                    // while this question is being asked, and no window is
                    // open yet. A seat being asked whether it *wants* a debt
                    // does not have one, so nothing is owed here.
                    assert_eq!(
                        seat_view(session, payer).owed,
                        None,
                        "a seat still deciding whether to pay owes nothing yet"
                    );
                    agreed = true;
                    PlayerAction::YesNo(true)
                }
                // Exactly one Plains is tapped, and only to make the spell
                // castable. Tapping the other two here would settle the tax
                // out of the pool and open no window at all, which is the
                // engine's own "nothing to press" branch and a different
                // game from this one.
                Pending::Priority { legal, .. } if seat == payer && !cast => {
                    if let Some(&card) = legal.castable.first() {
                        cast = true;
                        PlayerAction::CastSpell { card }
                    } else if let Some(&source) = legal.mana_abilities.first() {
                        PlayerAction::ActivateManaAbility { source }
                    } else {
                        PlayerAction::PassPriority
                    }
                }
                _ => PlayerAction::PassPriority,
            };
            if session.act(seat, action).is_err() {
                break;
            }
        }
        agreed
    }

    /// The seat inside a CR 605.3a payment window is told what it owes, and
    /// so is the seat watching it.
    ///
    /// The window is an ordinary `Pending::Priority` offering mana abilities
    /// and nothing else — deliberately, so that a client draws it and an
    /// agent answers it with no new question shape — which is exactly why
    /// nothing could tell it apart from a quiet priority pass with no plays.
    /// The house agent said yes to ward's tax, was handed the window, saw
    /// nothing castable over two untapped Plains, passed, and lost its own
    /// spell. `crates/baylee-gamehost/tests/ai_ward.rs` on the `ai` branch
    /// pins that from the agent's side; this is the half the view owes it.
    ///
    /// Asserted from the **bystander's** view as well as the payer's,
    /// because that is the vantage point where a missing field is visible:
    /// the pending question is sent only to the seat it is addressed to, so
    /// seat 1 has nothing else to read it off.
    #[test]
    fn a_seat_in_a_payment_window_is_told_what_it_owes() {
        use baylee_core::mana::ManaCost;

        let payer = PlayerId::new(0);
        let bystander = PlayerId::new(1);
        let mut session = Session::new(&a_table_with_a_warded_creature()).expect("preset builds");

        assert!(
            drive_into_ward_s_payment_window(&mut session, payer),
            "the removal was never cast at the warded creature"
        );

        let owed = ManaCost::from_symbol_generic(2);
        assert_eq!(
            session.awaiting_seat(),
            Some(payer),
            "the payer is the seat holding priority inside its own window"
        );
        assert_eq!(
            seat_view(&session, payer).owed,
            Some(owed),
            "a seat that has just agreed to pay is owed the number it agreed to"
        );
        assert_eq!(
            seat_view(&session, bystander).owed,
            Some(owed),
            "and so is the seat watching, which has no question to read it off"
        );

        // The number is usable, which is the whole claim: two taps and a pass
        // settle the tax, the window closes and the spell resolves.
        for _ in 0..4 {
            let Pending::Priority { player, legal } = session.pending().clone() else {
                break;
            };
            let Some(&source) = legal.mana_abilities.first() else {
                break;
            };
            session
                .act(player, PlayerAction::ActivateManaAbility { source })
                .expect("the window offers these and nothing else");
        }
        assert_eq!(
            seat_view(&session, payer).owed,
            Some(owed),
            "the total that was asked, not a remainder that shrinks as lands tap"
        );
        let _ = session.act(payer, PlayerAction::PassPriority);
        assert_eq!(
            seat_view(&session, bystander).owed,
            None,
            "a closed window owes nothing"
        );

        // Closing the window settles the tax; the spell under it is still on
        // the stack and resolves on the next round of passes.
        let throne = baylee_cards::decks::by_name("Roaming Throne").expect("compiled");
        for _ in 0..8 {
            let Some(seat) = session.awaiting_seat() else {
                break;
            };
            if !matches!(session.pending(), Pending::Priority { .. }) {
                break;
            }
            if session.act(seat, PlayerAction::PassPriority).is_err() {
                break;
            }
        }
        let state = session.state();
        assert!(
            !state
                .battlefield_view()
                .into_iter()
                .filter_map(|id| state.object(id))
                .any(|o| o.card.is_some_and(|c| c.index == throne)),
            "the tax was paid, so Swords to Plowshares resolved"
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
        let asked = session.pending().asked();
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
        let (player, action) = timeout(&session).expect("somebody is being asked something");
        assert_eq!(Some(player), session.awaiting_seat());

        let seq_before = session.seq();
        session
            .act(player, action)
            .expect("the timeout answer is legal");
        assert!(session.seq() > seq_before, "the game moved on");
    }

    /// The house that answers for a timed-out seat plays for that seat's
    /// team. It used to be built with no teams, so every other chair looked
    /// like an opponent. The engine never offers a teammate as a defender,
    /// but the agent's own reading of the table still counted one as a
    /// threat. Here the seat is on 5 life beside a teammate with eight hasty
    /// goblins, and the opponents have nothing: a house that fears the swing
    /// back from its own partner keeps its attackers home. `stand_in` built
    /// its agent with the table's teams all along; the clock's did not.
    ///
    /// Asked of [`Session::house_action`] since #258: the clock itself now
    /// attacks with nothing, and the house answers only what has no answer
    /// that does nothing. That house is still this one.
    #[test]
    fn a_clock_answer_does_not_fear_a_teammate() {
        use baylee_core::ids::Defender;
        let goblin = DeckEntry {
            card: baylee_cards::decks::by_name("Raging Goblin").expect("a hasty 1/1"),
            print: PrintRef::new(0),
        };
        let mut preset = teamed_preset([Some(1), Some(2), Some(1), Some(2)]);
        preset.seats[0].starting_battlefield = vec![goblin; 4];
        preset.seats[0].starting_life = Some(5);
        preset.seats[2].starting_battlefield = vec![goblin; 8];
        let mut session = Session::new(&preset).expect("session builds");
        let _ = session.pump();
        let me = PlayerId::new(0);
        for _ in 0..200 {
            let seat = session.awaiting_seat().expect("somebody is asked");
            let action = match session.engine.pending() {
                Pending::ChooseAttackers { .. } if seat == me => break,
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                Pending::Priority { .. } => PlayerAction::PassPriority,
                other => panic!("an unexpected question before the first attack: {other:?}"),
            };
            session.act(seat, action).expect("a legal answer");
        }
        assert!(
            matches!(session.engine.pending(), Pending::ChooseAttackers { .. }),
            "the table never reached the first seat's attack"
        );

        let (player, action) = house(&session).expect("the attack is asked");
        assert_eq!(player, me);
        let PlayerAction::DeclareAttackers { attackers } = action else {
            panic!("the clock answered an attack with {action:?}")
        };
        assert_eq!(
            attackers.len(),
            4,
            "the house kept goblins home against its own teammate: {attackers:?}"
        );
        assert!(
            attackers
                .iter()
                .all(|(_, defender)| *defender != Defender::Player(PlayerId::new(2))),
            "{attackers:?}"
        );
    }

    /// A seat that runs out of time at priority passes, with a spell it
    /// could cast in hand and the land to cast it (#258). Before, the house
    /// played the seat in full and cast the spell for it, which a player
    /// who stepped away finds done, and which no button could say ahead of
    /// time. The fixture is checked from the other side too: the house,
    /// asked the same question, does act, so a pass here is the clock's
    /// choice and not the only thing there was to do.
    #[test]
    fn a_seat_that_runs_out_of_time_at_priority_passes() {
        let elves_card = baylee_cards::decks::by_name("Llanowar Elves").expect("in the pool");
        let elves = DeckEntry {
            card: elves_card,
            print: PrintRef::new(0),
        };
        let forest = DeckEntry {
            card: baylee_cards::decks::by_name("Forest").expect("in the pool"),
            print: PrintRef::new(0),
        };
        let mut preset = test_preset();
        preset.seats[0].starting_hand = Some(vec![elves]);
        preset.seats[0].starting_battlefield = vec![forest];
        let mut session = Session::new(&preset).expect("session builds");
        let _ = session.pump();
        let me = PlayerId::new(0);
        let my_main = |session: &Session| {
            let turn = &session.engine.state().turn;
            session.awaiting_seat() == Some(me)
                && turn.active == me
                && turn.step == baylee_engine::turn::Step::Main
                && matches!(session.engine.pending(), Pending::Priority { .. })
        };
        for _ in 0..200 {
            if my_main(&session) {
                break;
            }
            let seat = session.awaiting_seat().expect("the game goes on");
            let action = match session.engine.pending() {
                Pending::Mulligan { .. } => PlayerAction::MulliganKeep,
                Pending::Priority { .. } => PlayerAction::PassPriority,
                other => panic!("an unexpected question before the first main phase: {other:?}"),
            };
            session.act(seat, action).expect("a legal answer");
        }
        assert!(
            my_main(&session),
            "the table never reached the seat's main phase"
        );
        let (_, house) = house(&session).expect("the seat is asked");
        assert_ne!(
            house,
            PlayerAction::PassPriority,
            "the house would pass here too, so this proves nothing"
        );
        assert_eq!(timeout(&session), Some((me, PlayerAction::PassPriority)));

        clock_answers(&mut session, me)
            .expect("the seat is being asked")
            .expect("passing is legal");
        let state = session.engine.state();
        let hand = state
            .zones
            .list(baylee_engine::zone::ZoneLocation::Hand(me));
        assert!(
            hand.iter().any(|&id| state
                .object(id)
                .and_then(|o| o.card)
                .is_some_and(|card| card.index == elves_card)),
            "the Elves are still in hand"
        );
        assert!(state.zones.stack_is_empty(), "nothing was cast");
        // The house's own first step here is tapping the Forest for the
        // Elves, which leaves both of the above true.
        let tapped: Vec<_> = state
            .zones
            .list(baylee_engine::zone::ZoneLocation::Battlefield)
            .iter()
            .filter_map(|&id| state.object(id))
            .filter(|o| {
                o.controller == me && o.status.contains(baylee_engine::object::Status::TAPPED)
            })
            .map(|o| o.id)
            .collect();
        assert!(tapped.is_empty(), "the clock tapped {tapped:?}");
    }

    /// The clock's order, against an engine that refuses doing nothing: the
    /// empty declaration first, and the house's own answer once it is
    /// refused. Written against a closure because no card in the pool can
    /// make that refusal today (see [`by_clock`]).
    #[test]
    fn a_refused_answer_that_does_nothing_falls_back_to_the_house() {
        use baylee_core::ids::{Defender, ObjectId};
        let attacker = ObjectId::new(3, 0);
        let pending = Pending::ChooseAttackers {
            player: PlayerId::new(0),
            attackers: vec![attacker],
            defenders: vec![Defender::Player(PlayerId::new(1))],
        };
        let nothing = PlayerAction::DeclareAttackers {
            attackers: Vec::new(),
        };
        let forced = PlayerAction::DeclareAttackers {
            attackers: vec![(attacker, Defender::Player(PlayerId::new(1)))],
        };
        // "Attacks each combat if able": the engine takes only the attack.
        let refusing = |tried: &mut Vec<PlayerAction>, action: PlayerAction| {
            tried.push(action.clone());
            if action == nothing {
                Err("the creature attacks if able".to_string())
            } else {
                Ok(())
            }
        };

        let mut tried = Vec::new();
        let out = by_clock(&mut tried, &pending, refusing, |_| Some(forced.clone()));
        assert_eq!(out, Some(Ok(())));
        assert_eq!(tried, vec![nothing.clone(), forced.clone()]);

        // Accepted, the house is never asked.
        let mut tried = Vec::new();
        let out = by_clock(
            &mut tried,
            &pending,
            |tried: &mut Vec<PlayerAction>, action| {
                tried.push(action);
                Ok::<(), String>(())
            },
            |_| panic!("the house was asked although doing nothing was accepted"),
        );
        assert_eq!(out, Some(Ok(())));
        assert_eq!(tried, vec![nothing]);

        // A question with no answer that does nothing goes to the house at once.
        let discard = Pending::DiscardChoice {
            player: PlayerId::new(0),
            count: 1,
        };
        let chosen = PlayerAction::ChooseObjects {
            objects: vec![attacker],
        };
        let mut tried = Vec::new();
        let out = by_clock(
            &mut tried,
            &discard,
            |tried: &mut Vec<PlayerAction>, action| {
                tried.push(action);
                Ok::<(), String>(())
            },
            |_| Some(chosen.clone()),
        );
        assert_eq!(out, Some(Ok(())));
        assert_eq!(tried, vec![chosen]);
    }

    /// Answers for every seat but `seat` until `seat` is asked, as the
    /// players at those seats would: with the house's legal answer, through
    /// the socket door, so none of them is marked as answered by the house.
    fn until_asked(session: &mut Session, seat: PlayerId) {
        for _ in 0..200 {
            let asked = session.awaiting_seat().expect("the game goes on");
            if asked == seat {
                return;
            }
            let (player, action) = timeout(session).expect("a question is out");
            session.act(player, action).expect("a legal answer");
        }
        panic!("seat {seat:?} was never asked");
    }

    /// The clock's answer is marked on the seat it answered for, in the very
    /// views that answer sends out. An automation setting from the seat
    /// afterwards is not an answer and leaves the mark; the seat's own next
    /// decision clears it.
    #[test]
    fn the_clock_marks_the_seat_until_it_answers_itself() {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let _ = session.pump();
        let me = PlayerId::new(0);
        until_asked(&mut session, me);

        let routed = clock_answers(&mut session, me)
            .expect("the seat is being asked")
            .expect("the house's answer is legal");
        let seats = routed_view(&routed, me).seats;
        assert_eq!(seats[0].house_answered, Some(HouseAnswer::Clock));
        assert_eq!(seats[1].house_answered, None, "an AI chair is never marked");

        let turn = session.engine.state().turn.number;
        let routed = session
            .act(
                me,
                PlayerAction::SetPriorityHold(
                    baylee_engine::choice::PriorityHold::UntilEndOfTurn { turn },
                ),
            )
            .expect("a seat may state a standing order at any time");
        assert_eq!(
            routed_view(&routed, me).seats[0].house_answered,
            Some(HouseAnswer::Clock),
            "an automation setting is not the seat answering"
        );

        until_asked(&mut session, me);
        let (_, action) = timeout(&session).expect("the seat is asked");
        let routed = session.act(me, action).expect("a legal answer");
        assert_eq!(routed_view(&routed, me).seats[0].house_answered, None);
    }

    /// A timer that fires after its question has moved on answers nothing:
    /// one seat's expired clock never takes another seat's decision.
    #[test]
    fn the_clock_answers_nothing_for_a_seat_that_is_not_asked() {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let _ = session.pump();
        let me = PlayerId::new(0);
        until_asked(&mut session, me);
        let before = session.decision_seq();
        assert!(clock_answers(&mut session, PlayerId::new(1)).is_none());
        assert_eq!(session.decision_seq(), before);
        assert_eq!(seat_view(&session, me).seats[0].house_answered, None);
    }

    /// The house standing in for an absent player marks the chair, and the
    /// player coming back does not clear it: the last decision is still the
    /// house's until the player makes one.
    #[test]
    fn a_stand_in_marks_the_seat_and_a_reconnect_leaves_the_mark() {
        let mut preset = test_preset();
        preset.seats[1].controller = SeatController::Open;
        let mut session = Session::new(&preset).expect("session builds");
        let _ = session.pump();
        let (me, other) = (PlayerId::new(0), PlayerId::new(1));
        until_asked(&mut session, me);

        assert!(session.stand_in(me));
        let routed = session.pump();
        assert_eq!(
            routed_view(&routed, other).seats[0].house_answered,
            Some(HouseAnswer::StandIn),
            "the table is told the house played that chair"
        );
        assert!(session.hand_back(me));
        assert_eq!(
            seat_view(&session, other).seats[0].house_answered,
            Some(HouseAnswer::StandIn)
        );

        until_asked(&mut session, me);
        let (_, action) = timeout(&session).expect("the seat is asked");
        let routed = session.act(me, action).expect("a legal answer");
        assert_eq!(routed_view(&routed, other).seats[0].house_answered, None);
    }

    /// An AI chair is the house's to play, driven or not, so the clock
    /// answering for a driven one marks nothing: the roster already says it.
    #[test]
    fn the_clock_does_not_mark_a_driven_ai_chair() {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let driven = PlayerId::new(1);
        assert!(session.take_over(driven));
        let _ = session.pump();
        until_asked(&mut session, driven);
        let routed = clock_answers(&mut session, driven)
            .expect("the seat is being asked")
            .expect("the house's answer is legal");
        assert_eq!(routed_view(&routed, driven).seats[1].house_answered, None);
    }

    /// Answering by timeout over and over drives the game forward rather than
    /// deadlocking on a pending nobody can satisfy.
    #[test]
    fn repeated_timeouts_keep_the_game_moving() {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let _ = session.pump();
        let mut answered = 0;
        for _ in 0..40 {
            let Some((player, action)) = timeout(&session) else {
                break;
            };
            if session.act(player, action).is_err() {
                break;
            }
            answered += 1;
        }
        assert!(answered > 5, "only {answered} timeouts were answered");
    }
    #[test]
    fn scouting_is_revoked_on_takeover_and_never_granted_to_human_standins() {
        use baylee_ai::intelligence::{LibraryAccess, ScoutingRequest};
        let mut preset = split_preset();
        preset.seats[0].controller = SeatController::Open;
        preset.seats[1].controller = SeatController::Ai(AIProfile::EXPERT);
        preset.seats[1].sideboard = vec![DeckEntry {
            card: forest(),
            print: PrintRef::new(1),
        }];
        let mut session = Session::new(&preset).unwrap();
        let request = ScoutingRequest {
            opponents: true,
            hands: true,
            library: LibraryAccess::All,
            sideboards: true,
        };
        let scout = |session: &Session, player| {
            crate::scouting::request(
                &session.seats,
                &session.scouting_decks,
                session.engine.state(),
                player,
                request,
            )
            .is_some()
        };
        assert!(scout(&session, PlayerId::new(1)));
        assert!(!scout(&session, PlayerId::new(0)));
        assert!(!scout(&session, PlayerId::new(250)));
        let before = serde_json::to_vec(&session.game_static(PlayerId::new(0))).unwrap();
        let report = crate::scouting::request(
            &session.seats,
            &session.scouting_decks,
            session.engine.state(),
            PlayerId::new(1),
            request,
        )
        .unwrap();
        assert_eq!(report.seats.len(), 2);
        for seat in &report.seats {
            let p = usize::from(seat.player.get());
            assert_eq!(
                seat.deck.cards,
                preset.seats[p]
                    .deck
                    .iter()
                    .map(|e| e.card)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                seat.library.as_ref().unwrap().len(),
                session
                    .engine
                    .state()
                    .zones
                    .list(baylee_engine::zone::ZoneLocation::Library(seat.player))
                    .len()
            );
        }
        assert_eq!(
            report.seats[1].sideboard.as_deref(),
            Some([forest()].as_slice())
        );
        assert_eq!(
            serde_json::to_vec(&session.game_static(PlayerId::new(0))).unwrap(),
            before,
            "scouting must not teach human sockets any print identity"
        );
        assert!(session.take_over(PlayerId::new(1)));
        assert!(!scout(&session, PlayerId::new(1)));
        assert!(session.release(PlayerId::new(1)));
        assert!(scout(&session, PlayerId::new(1)));
        assert!(session.stand_in(PlayerId::new(0)));
        assert!(!scout(&session, PlayerId::new(0)));
        assert!(session.hand_back(PlayerId::new(0)));
        assert!(!scout(&session, PlayerId::new(0)));
    }

    #[test]
    fn scouting_top_cards_are_bounded_ordered_and_do_not_change_human_views() {
        use baylee_ai::intelligence::{LibraryAccess, ScoutingRequest};
        let mut preset = test_preset();
        preset.seats[1].deck[2].card = forest();
        preset.seats[1].deck[8].card = forest();
        let session = Session::new(&preset).unwrap();
        let before = crate::view::player_view(
            session.engine.state(),
            PlayerId::new(0),
            0,
            None,
            &crate::view::SeatContext::default(),
            &[],
        );
        let report = crate::scouting::request(
            &session.seats,
            &session.scouting_decks,
            session.engine.state(),
            PlayerId::new(1),
            ScoutingRequest {
                library: LibraryAccess::Top(3),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(report.seats.len(), 1);
        assert!(report.seats[0].hand.is_none());
        assert!(report.seats[0].sideboard.is_none());
        let expected: Vec<_> = session
            .engine
            .state()
            .zones
            .list(baylee_engine::zone::ZoneLocation::Library(PlayerId::new(1)))
            .iter()
            .rev()
            .take(3)
            .map(|id| {
                session
                    .engine
                    .state()
                    .object(*id)
                    .unwrap()
                    .card
                    .unwrap()
                    .index
            })
            .collect();
        assert_eq!(report.seats[0].library.as_ref().unwrap(), &expected);
        assert_eq!(
            before,
            crate::view::player_view(
                session.engine.state(),
                PlayerId::new(0),
                0,
                None,
                &crate::view::SeatContext::default(),
                &[]
            )
        );
    }

    /// Two human seats, both on a 30-second decision clock.
    fn two_humans() -> GamePreset {
        let mut preset = test_preset();
        preset.seats[1].controller = SeatController::Open;
        preset.house_rules.decision_timeout_secs = 30;
        preset
    }

    /// Every question `routed` asks, by seat.
    fn asked(routed: &[(PlayerId, Envelope)]) -> Vec<(u8, Pending)> {
        routed
            .iter()
            .filter_map(|(seat, env)| match &env.msg {
                Some(v1::envelope::Msg::ChoiceRequest(req)) => Some((
                    seat.get(),
                    serde_json::from_slice(&req.pending_json).expect("the question decodes"),
                )),
                _ => None,
            })
            .collect()
    }

    /// Every seat is asked its opening mulligan at once, each only its own,
    /// and a seat that comes back mid-window is asked its own again.
    #[test]
    fn every_human_seat_is_asked_its_own_mulligan_at_once() {
        let mut session = Session::new(&two_humans()).expect("session builds");
        let routed = session.pump();
        let questions = asked(&routed);
        assert_eq!(questions.len(), 2, "{questions:?}");
        for (seat, pending) in &questions {
            assert!(
                matches!(pending, Pending::Mulligan { player, .. } if player.get() == *seat),
                "seat {seat} was asked {pending:?}"
            );
        }
        let me = PlayerId::new(1);
        let resumed = asked(
            &session
                .snapshot(me)
                .into_iter()
                .map(|env| (me, env))
                .collect::<Vec<_>>(),
        );
        assert!(
            matches!(resumed.as_slice(), [(1, Pending::Mulligan { player, .. })] if *player == me),
            "{resumed:?}"
        );
    }

    /// An AI chair keeps while a human is still deciding: nobody waits on
    /// anybody before turn 1.
    #[test]
    fn an_ai_chair_keeps_while_a_human_is_still_deciding() {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let routed = session.pump();
        assert_eq!(session.awaited(), [PlayerId::new(0)].into_iter().collect());
        assert!(session.engine.pending_for(PlayerId::new(1)).is_none());
        assert!(
            matches!(asked(&routed).as_slice(), [(0, Pending::Mulligan { .. })]),
            "{:?}",
            asked(&routed)
        );
    }

    /// One seat's mulligan answer asks no other seat anything new, so the
    /// other seat's clock runs on: its anchor stays, its reading is what it
    /// is shown, and its deadline still answers for it (a keep, #258).
    #[test]
    fn one_seats_mulligan_answer_leaves_the_other_seats_clock_running() {
        let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
        for answer in [PlayerAction::MulliganTake, PlayerAction::MulliganKeep] {
            let mut session = Session::new(&two_humans()).expect("session builds");
            let _ = session.pump();
            let at_zero = session.asked_at(zero).expect("seat 0 is asked");
            let at_one = session.asked_at(one).expect("seat 1 is asked");
            session.set_decision_remaining(zero, at_zero, Some(9_000));
            session.set_decision_remaining(one, at_one, Some(4_000));

            session.act(zero, answer.clone()).expect("seat 0 answers");
            assert_eq!(session.asked_at(one), Some(at_one), "after {answer:?}");
            assert_eq!(
                session.decision_remaining_ms(one),
                Some(4_000),
                "seat 1's clock started over after seat 0's {answer:?}"
            );
            assert!(
                clock_answers(&mut session, zero)
                    .is_none_or(|_| session.asked_at(zero) != Some(at_zero)),
                "seat 0's old deadline still answered for it"
            );
            assert!(
                session
                    .answer_by_clock(one, at_one)
                    .is_some_and(|r| r.is_ok()),
                "seat 1's deadline did not answer after seat 0's {answer:?}"
            );
            assert!(
                session.engine.pending_for(one).is_none(),
                "seat 1 did not keep when its clock ran out"
            );
        }
    }

    /// A deadline armed for a question that has since moved on answers
    /// nothing: the seat's own answer moves its anchor.
    #[test]
    fn a_deadline_for_a_question_already_answered_answers_nothing() {
        let zero = PlayerId::new(0);
        let mut session = Session::new(&two_humans()).expect("session builds");
        let _ = session.pump();
        let at_zero = session.asked_at(zero).expect("seat 0 is asked");
        session
            .act(zero, PlayerAction::MulliganTake)
            .expect("seat 0 takes");
        assert_ne!(session.asked_at(zero), Some(at_zero));
        assert!(session.answer_by_clock(zero, at_zero).is_none());
        assert!(
            matches!(
                session.engine.pending_for(zero),
                Some(Pending::Mulligan { taken: 1, .. })
            ),
            "the stale deadline answered seat 0's new question"
        );
    }

    /// Outside the mulligans a bystander's move asks the awaited seat anew,
    /// as it did when one count was every clock's anchor. A concession is
    /// the one such move (only the seat with priority may offer a draw).
    /// Today the engine hands seat 0's priority on to seat 1 when seat 2
    /// concedes (#275); once seat 0 keeps it, as CR 800.4a has it, this is a
    /// seat asked anew without having answered, and the same assertions hold.
    #[test]
    fn a_bystanders_concession_asks_the_awaited_seat_anew() {
        let (zero, two) = (PlayerId::new(0), PlayerId::new(2));
        let mut preset = two_humans();
        preset.seats.push(preset.seats[1].clone());
        let mut session = Session::new(&preset).expect("session builds");
        let _ = session.pump();
        for seat in 0..3 {
            session
                .act(PlayerId::new(seat), PlayerAction::MulliganKeep)
                .expect("keeps");
        }
        until_asked(&mut session, zero);
        let before = session.asked_at(zero).expect("seat 0 is asked");
        session.set_decision_remaining(zero, before, Some(4_000));
        session
            .act(two, PlayerAction::Concede)
            .expect("seat 2 may concede");
        let asked = session.awaiting_seat().expect("the game goes on");
        assert_eq!(session.asked_at(asked), Some(session.decision_seq()));
        assert_eq!(session.decision_remaining_ms(asked), Some(30_000));
    }

    // ---- the game log (#262)

    /// Every state frame `routed` sends `seat`, in order: its `seq`, its view
    /// as sent, and its log tail when it carries one.
    fn state_frames_to(
        routed: &[(PlayerId, Envelope)],
        seat: PlayerId,
    ) -> Vec<(u64, Vec<u8>, Option<LogTail>)> {
        routed
            .iter()
            .filter(|(to, _)| *to == seat)
            .filter_map(|(_, env)| match &env.msg {
                Some(v1::envelope::Msg::StateDelta(delta)) => Some((
                    delta.seq,
                    delta.view_json.clone(),
                    (!delta.log_json.is_empty())
                        .then(|| serde_json::from_slice(&delta.log_json).expect("the log decodes")),
                )),
                _ => None,
            })
            .collect()
    }

    /// The tails in `frames`, checked to be what a client appending by
    /// `from` needs: the first starts at `held`, each starts where the one
    /// before ended, and none is longer than a frame may carry. Returns every
    /// line they carry, in order.
    fn contiguous_from(held: usize, frames: &[(u64, Vec<u8>, Option<LogTail>)]) -> Vec<LogEntry> {
        let mut next = held;
        let mut lines = Vec::new();
        for tail in frames.iter().filter_map(|(_, _, tail)| tail.as_ref()) {
            assert_eq!(tail.from as usize, next, "a gap or an overlap in one pump");
            assert!(
                !tail.entries.is_empty(),
                "a tail that says nothing is sent as none"
            );
            assert!(
                tail.entries.len() <= LOG_TAIL_CAP,
                "{} lines in one frame",
                tail.entries.len()
            );
            next += tail.entries.len();
            lines.extend(tail.entries.iter().cloned());
        }
        lines
    }

    /// Lines enough that no one frame may carry them, each different from the
    /// last so that none folds into another.
    fn overflow(session: &mut Session, player: PlayerId) -> usize {
        let lines = LOG_TAIL_CAP * 2 + 3;
        for result in 0..lines {
            session.log.note(LogEvent::DiceRolled {
                player,
                sides: 1_000,
                result: u32::try_from(result).expect("a small number"),
            });
        }
        lines
    }

    /// More lines than one frame may carry, pending at once mid-game, arrive
    /// over several frames in one pump, every line once and in order. Every
    /// frame is a whole view, the same one at the same `seq`, so a client
    /// that drops all but the first as not newer still reads every line; and
    /// the pump after sends none of them again.
    #[test]
    fn an_overflowing_log_arrives_once_and_in_order_mid_game() {
        let (mut session, human) = started_session();
        let _ = session.pump();
        let held = session.told[human.get() as usize];
        assert_eq!(held, session.log.len(), "the human holds every line so far");
        let added = overflow(&mut session, human);

        let routed = session.pump();
        let frames = state_frames_to(&routed, human);
        assert_eq!(frames.len(), added.div_ceil(LOG_TAIL_CAP), "{frames:?}");
        assert!(
            frames
                .iter()
                .all(|(seq, view, _)| (*seq, view) == (frames[0].0, &frames[0].1)),
            "every frame repeats the one view"
        );
        let lines = contiguous_from(held, &frames);
        assert_eq!(held + lines.len(), session.log.len(), "every line arrived");
        assert_eq!(lines, session.log.told(human, held, session.log.len()));

        let again = session.pump();
        assert!(
            state_frames_to(&again, human)
                .iter()
                .all(|(_, _, tail)| tail.is_none()),
            "a line is sent once"
        );
    }

    /// The same at the end of the game, where no later view could carry what
    /// is left: the frames that tell the seat the game is over carry every
    /// line, down to the last.
    #[test]
    fn an_overflowing_log_arrives_whole_at_game_over() {
        let (mut session, human) = started_session();
        let _ = session.pump();
        let held = session.told[human.get() as usize];
        overflow(&mut session, human);

        let routed = session
            .act(human, PlayerAction::Concede)
            .expect("a player may always concede");
        assert!(matches!(session.pending(), Pending::GameOver(_)));
        let lines = contiguous_from(held, &state_frames_to(&routed, human));
        assert_eq!(held + lines.len(), session.log.len(), "every line arrived");
        assert_eq!(lines, session.log.told(human, held, session.log.len()));
        assert!(
            matches!(
                lines.last().map(|line| &line.event),
                Some(LogEvent::GameOver { winners }) if winners.contains(PlayerId::new(1))
            ),
            "the last line is the end: {:?}",
            lines.last()
        );
        let after = session.pump();
        assert!(
            state_frames_to(&after, human)
                .iter()
                .all(|(_, _, tail)| tail.is_none()),
            "and nothing is left to send"
        );
    }

    /// An AI seat's agent is handed a view and a question, never a log, and
    /// its chair is never sent a frame. Pinned here rather than left to the
    /// view not having a log field: the one door a log leaves by refuses
    /// every seat nobody answers over a socket, a chair the house holds for
    /// an absent player included.
    #[test]
    fn a_seat_the_house_answers_is_never_told_the_log() {
        let (mut session, human) = started_session();
        let ai = PlayerId::new(1);
        for _ in 0..40 {
            let Some(action) = session.timeout_action(human) else {
                break;
            };
            let routed = session.act(human, action).expect("a legal answer");
            assert!(
                routed.iter().all(|(to, _)| *to != ai),
                "a frame was sent to the AI chair"
            );
        }
        assert!(session.log.len() > 10, "the game was logged");
        assert_eq!(session.told[1], 0, "no line was ever counted as sent to it");
        assert_eq!(
            session.log_tail(ai),
            LogTail::default(),
            "asked outright, it tells nothing"
        );
        assert_eq!(session.log_sent(ai), LogTail::default());

        let mut session = Session::new(&two_humans()).expect("session builds");
        let _ = session.pump();
        assert!(session.stand_in(ai));
        assert_eq!(
            session.log_tail(ai),
            LogTail::default(),
            "nor to a chair the house holds"
        );
    }

    /// A socket that attaches again holds none of the log, so its first view
    /// carries all of it.
    #[test]
    fn a_socket_that_attaches_again_is_told_the_log_from_the_first_line() {
        let (mut session, human) = started_session();
        let _ = session.pump();
        let lines = session.log.len();
        assert!(lines > 0);

        session.retell_log(human);
        let frames = state_frames_to(&session.pump(), human);
        let told = contiguous_from(0, &frames);
        assert_eq!(told, session.log.told(human, 0, lines));
    }

    /// A rebuild after lost frames carries every line the seat was sent, and
    /// marks nothing; a refused answer is handed its question back with no
    /// log at all.
    #[test]
    fn a_snapshot_carries_the_lines_sent_and_a_reask_none() {
        let (mut session, human) = started_session();
        let _ = session.pump();
        let sent = session.told[human.get() as usize];
        session.log.note(LogEvent::Shuffled { player: human });

        let snapshot = session.snapshot(human);
        let rebuilt = contiguous_from(
            0,
            &state_frames_to(
                &snapshot
                    .iter()
                    .map(|env| (human, env.clone()))
                    .collect::<Vec<_>>(),
                human,
            ),
        );
        assert_eq!(
            rebuilt,
            session.log.told(human, 0, sent),
            "the lines sent, and not the one still to go"
        );
        assert_eq!(
            session.told[human.get() as usize],
            sent,
            "a snapshot marks nothing sent"
        );

        let reask: Vec<_> = session
            .reask(human)
            .into_iter()
            .map(|env| (human, env))
            .collect();
        let frames = state_frames_to(&reask, human);
        assert_eq!(frames.len(), 1);
        assert!(frames[0].2.is_none(), "a refusal lost no line");
        assert_eq!(asked(&reask).len(), 1, "and the question is handed back");
    }

    /// The opening mulligans are logged by their counts, which are public,
    /// and nothing names a card before the first turn.
    #[test]
    fn the_opening_mulligans_are_logged_as_counts() {
        let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
        let mut session = Session::new(&two_humans()).expect("session builds");
        let _ = session.pump();
        // Two, so that at least one is not free and the keep puts a card
        // on the bottom.
        for _ in 0..2 {
            session
                .act(zero, PlayerAction::MulliganTake)
                .expect("a mulligan");
        }
        session
            .act(zero, PlayerAction::MulliganKeep)
            .expect("a keep");
        let Some(Pending::MulliganBottom { count, .. }) = session.engine.pending_for(zero) else {
            panic!("the keep puts a card on the bottom")
        };
        let bottom = usize::from(*count);
        let hand = session.state().zones.list(ZoneLocation::Hand(zero))[..bottom].to_vec();
        session
            .act(zero, PlayerAction::ChooseObjects { objects: hand })
            .expect("the bottom");
        session
            .act(one, PlayerAction::MulliganKeep)
            .expect("a keep");

        let lines: Vec<LogEvent> = session
            .log
            .told(one, 0, session.log.len())
            .into_iter()
            .map(|line| line.event)
            .collect();
        let start = lines
            .iter()
            .position(|event| matches!(event, LogEvent::TurnStarted { .. }))
            .expect("the first turn began");
        assert_eq!(
            lines[..start],
            [
                LogEvent::Mulliganed { player: zero },
                LogEvent::Mulliganed { player: zero },
                LogEvent::Kept {
                    player: zero,
                    cards: u8::try_from(7 - bottom).expect("a hand")
                },
                LogEvent::Kept {
                    player: one,
                    cards: 7
                },
            ]
        );
    }

    /// A decision clock's answer is logged as what the clock did.
    #[test]
    fn a_clock_answer_is_logged_as_what_the_clock_did() {
        let mut session = Session::new(&test_preset()).expect("session builds");
        let _ = session.pump();
        let me = PlayerId::new(0);
        clock_answers(&mut session, me)
            .expect("seat 0 owes its mulligan")
            .expect("the clock keeps");
        until_asked(&mut session, me);
        assert!(matches!(session.pending(), Pending::Priority { .. }));
        clock_answers(&mut session, me)
            .expect("seat 0 has priority")
            .expect("the clock passes");
        let timed_out: Vec<LogEvent> = session
            .log
            .told(me, 0, session.log.len())
            .into_iter()
            .map(|line| line.event)
            .filter(|event| matches!(event, LogEvent::TimedOut { .. }))
            .collect();
        assert_eq!(
            timed_out,
            [
                LogEvent::TimedOut {
                    player: me,
                    answer: ClockAnswer::Kept
                },
                LogEvent::TimedOut {
                    player: me,
                    answer: ClockAnswer::Passed
                },
            ]
        );
    }

    /// Every answer that does nothing is logged by its name, and anything
    /// else the clock answered is the house's choice.
    #[test]
    fn every_answer_that_does_nothing_is_logged_by_name() {
        let me = PlayerId::new(0);
        let questions = [
            (
                Pending::Priority {
                    player: me,
                    legal: Box::default(),
                },
                ClockAnswer::Passed,
            ),
            (
                Pending::Mulligan {
                    player: me,
                    taken: 0,
                    next_is_free: false,
                },
                ClockAnswer::Kept,
            ),
            (
                Pending::ChooseAttackers {
                    player: me,
                    attackers: Vec::new(),
                    defenders: Vec::new(),
                },
                ClockAnswer::NoAttackers,
            ),
            (
                Pending::ChooseBlockers {
                    player: me,
                    attacker: PlayerId::new(1),
                    blockers: Vec::new(),
                },
                ClockAnswer::NoBlockers,
            ),
            (
                Pending::YesNo {
                    player: me,
                    prompt: baylee_engine::choice::YesNoPrompt::Kicker,
                    source: None,
                },
                ClockAnswer::Declined,
            ),
        ];
        for (pending, named) in questions {
            let quiet = baylee_engine::choice::timeout_answer(&pending).expect("a quiet answer");
            assert_eq!(clock_answer(Some(&pending), &quiet), named, "{pending:?}");
        }
        let priority = Pending::Priority {
            player: me,
            legal: Box::default(),
        };
        assert_eq!(
            clock_answer(Some(&priority), &PlayerAction::Concede),
            ClockAnswer::ChosenForThem
        );
    }

    /// A chair the house holds for an absent player is logged, and so is the
    /// player's return.
    #[test]
    fn a_stand_in_and_the_return_are_logged() {
        let one = PlayerId::new(1);
        let mut session = Session::new(&two_humans()).expect("session builds");
        assert!(session.stand_in(one));
        assert!(session.hand_back(one));
        let lines: Vec<LogEvent> = session
            .log
            .told(one, 0, session.log.len())
            .into_iter()
            .map(|line| line.event)
            .collect();
        assert_eq!(
            lines,
            [
                LogEvent::StandIn { player: one },
                LogEvent::Returned { player: one }
            ]
        );
    }

    /// A printing a seat meets only in its log is earned the same way as one
    /// met in its view: the entry arrives before the frame that points at it.
    #[test]
    fn a_printing_first_named_by_the_log_arrives_before_the_line() {
        let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
        let mut preset = split_preset();
        preset.seats[1].starting_battlefield = vec![];
        preset.seats[1].capabilities.dev_commands = true;
        let mut session = Session::new(&preset).expect("session");
        let _ = session.pump();
        assert!(session.game_static(zero).print(PrintRef::new(1)).is_none());

        let forest = session.state().zones.list(ZoneLocation::Library(one))[0];
        let state = session.engine.dev_state_mut(one).expect("dev commands");
        state
            .journal
            .record(baylee_engine::event::GameEvent::Revealed {
                player: one,
                cards: vec![forest],
            });
        session.log.consume(session.engine.state());
        let routed = session.pump();

        let to_zero: Vec<&Envelope> = routed
            .iter()
            .filter(|(seat, _)| *seat == zero)
            .map(|(_, env)| env)
            .collect();
        let statics = to_zero
            .iter()
            .position(|env| matches!(env.msg, Some(v1::envelope::Msg::GameStatic(_))));
        let line = to_zero.iter().position(|env| {
            matches!(&env.msg, Some(v1::envelope::Msg::StateDelta(delta)) if !delta.log_json.is_empty())
        });
        assert!(line.is_some(), "the reveal was sent");
        assert!(
            statics < line,
            "the print entry arrives before the line naming it"
        );
        assert!(session.game_static(zero).print(PrintRef::new(1)).is_some());
    }

    /// What the log measurement below adds up.
    #[derive(Clone, Copy, Debug, Default)]
    struct LogStats {
        games: u64,
        lines: u64,
        /// Pumps that sent the watching seat a view.
        sends: u64,
        /// Of those, the ones whose lines needed more than one frame.
        overflowing: u64,
        /// The most lines one pump sent it.
        largest: usize,
        naming: crate::log::Naming,
    }

    impl LogStats {
        fn add(&mut self, other: &Self) {
            self.games += other.games;
            self.lines += other.lines;
            self.sends += other.sends;
            self.overflowing += other.overflowing;
            self.largest = self.largest.max(other.largest);
            self.naming.references += other.naming.references;
            self.naming.unnamed += other.naming.unnamed;
            self.naming.dropped += other.naming.dropped;
            self.naming.abilities += other.naming.abilities;
            self.naming.by_source += other.naming.by_source;
        }
    }

    /// One game of `preset` with seat 0 a player the house answers for, as a
    /// player would: sent a view after every pump, as a socket is.
    fn play_logged(preset: &GamePreset, answers: usize) -> LogStats {
        let mut preset = preset.clone();
        preset.seats[0].controller = SeatController::Open;
        let mut stats = LogStats::default();
        let Some(mut session) = Session::new(&preset) else {
            return stats;
        };
        let me = PlayerId::new(0);
        let mut routed = session.pump();
        for _ in 0..answers {
            let frames = state_frames_to(&routed, me);
            let lines: usize = frames
                .iter()
                .filter_map(|(_, _, tail)| tail.as_ref())
                .map(|tail| tail.entries.len())
                .sum();
            if !frames.is_empty() {
                stats.sends += 1;
            }
            if lines > LOG_TAIL_CAP {
                stats.overflowing += 1;
            }
            stats.largest = stats.largest.max(lines);
            if matches!(session.pending(), Pending::GameOver(_)) {
                break;
            }
            let Some(action) = session.house_action(me) else {
                routed = session.pump();
                continue;
            };
            routed = match session.act(me, action) {
                Ok(routed) => routed,
                Err(_) => match session.timeout_action(me) {
                    Some(action) => session.act(me, action).unwrap_or_default(),
                    None => break,
                },
            };
        }
        stats.games = 1;
        stats.lines = session.log.len() as u64;
        stats.naming = session.log.naming();
        stats
    }

    /// How well the log names what it names, and how often a seat has more
    /// lines waiting than one frame carries, over self-play: the acceptance
    /// decks, and one game per implemented card. #262's handshake reports
    /// these numbers; the assertion is only that games were played.
    #[test]
    #[ignore = "one game per implemented card; minutes, not seconds"]
    fn the_log_measured_over_self_play() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../data/acceptance-decks.txt"
        ))
        .expect("acceptance deck file");
        let allytifact =
            baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact loads");
        let victory =
            baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory loads");
        let mut acceptance = LogStats::default();
        for seed in 1..=10 {
            acceptance.add(&play_logged(
                &baylee_cards::decks::preset_for(seed, &allytifact, &victory),
                2_000,
            ));
            acceptance.add(&play_logged(
                &baylee_cards::decks::preset_for(seed, &victory, &allytifact),
                2_000,
            ));
        }

        let cards: Vec<&'static baylee_cards_dsl::CardDef> =
            baylee_cards::all().filter(|d| d.is_implemented()).collect();
        let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
        let chunk = cards.len().div_ceil(threads).max(1);
        let pool = std::thread::scope(|scope| {
            let handles: Vec<_> = cards
                .chunks(chunk)
                .map(|slice| {
                    scope.spawn(move || {
                        let mut stats = LogStats::default();
                        for def in slice {
                            if let Some(preset) = baylee_cards::decks::probe_preset(9, def.index) {
                                stats.add(&play_logged(&preset, 400));
                            }
                        }
                        stats
                    })
                })
                .collect();
            let mut total = LogStats::default();
            for handle in handles {
                total.add(&handle.join().expect("a probe chunk does not panic"));
            }
            total
        });
        eprintln!("acceptance: {acceptance:?}");
        eprintln!("pool probes: {pool:?}");
        assert!(acceptance.games > 0 && pool.games > 0);
    }
}
