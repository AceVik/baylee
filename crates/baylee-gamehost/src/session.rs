//! One game session: an engine plus seats (humans and AI). Socket-free so
//! tests and both servers (engine-server dev harness, gateway) drive it
//! directly; transport lives with the callers.

use baylee_ai::{HeuristicAgent, pending_player, policy_seed};
use baylee_cards::dsl::CardDef;
use baylee_core::ids::{CardIndex, PlayerId};
use baylee_core::preset::{AIProfile, GamePreset, HouseRules, PrintInfo, SeatController};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::Engine;
use baylee_engine::state::CardLookup;
use baylee_protocol::v1::{self, Envelope};
use baylee_view::{GameStatic, HouseAnswer, SeatIdentity};

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
    /// The last decision-clock reading the caller took, and the question it
    /// was taken for.
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
    clock: Option<(u64, Option<u32>)>,
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
        Some(Self {
            engine,
            seats,
            scouting_decks: crate::scouting::decks(preset),
            seq: 0,
            decisions: 0,
            house_rules: preset.house_rules.clone(),
            clock: None,
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
    /// the first to show it one of the game's printings, or when a chair has
    /// changed hands since this seat was last told the roster.
    ///
    /// The order matters: the entry has to be there before the object that
    /// points at it, or the client draws a card it cannot key an image on.
    fn view_envelopes(&mut self, seat: PlayerId, awaiting: Option<PlayerId>) -> Vec<Envelope> {
        let view = crate::view::player_view(
            self.engine.state(),
            seat,
            self.seq,
            Some(self.engine.pending()),
            &crate::view::SeatContext {
                awaiting,
                held: self.engine.automation(seat).hold.suppresses(),
                owed: crate::view::owed_payment(&self.engine),
                decision_remaining_ms: self.decision_remaining_ms(),
            },
            &self.house_answered,
        );
        let mut out = Vec::new();
        // Two separate `let`s: `reveal` marks printings as shown, so folding
        // it into an `||` would let the other half short-circuit it away.
        let revealed = self.reveal(seat, &view);
        let roster_moved = self
            .roster_dirty
            .get(seat.get() as usize)
            .copied()
            .unwrap_or(false);
        if revealed || roster_moved {
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
                let awaiting = pending_player(&pending);
                for seat in self.human_seats() {
                    let envelopes = self.view_envelopes(seat, awaiting);
                    out.extend(envelopes.into_iter().map(|env| (seat, env)));
                }
                out.push((player, choice_envelope(self.seq, &pending)));
                return out;
            }
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
                            awaiting: pending_player(&pending),
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
            let moves_the_game = self.apply_house_action(player, action);
            self.seq += 1;
            if moves_the_game {
                self.decisions += 1;
                self.house_answered[player.get() as usize] = by;
            }
        }
        out
    }

    /// An invalid agent proposal must not leave an untimed seat stalled (#180).
    fn apply_house_action(&mut self, player: PlayerId, action: PlayerAction) -> bool {
        let moves = !action.is_automation_setting();
        if self.engine.apply(player, action).is_ok() {
            return moves;
        }
        // Re-read the actual question: a rejected proposal can have entered
        // a casting/payment wizard. The ordinary timeout policy covers all
        // question kinds instead of a positive list containing only Priority.
        let (seat, fallback) = self
            .timeout_action()
            .expect("refused AI action left no decision");
        self.engine.apply(seat, fallback).expect(
            "both AI proposal and recovery were refused; refusing to silently stall the table",
        );
        true
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

    /// Tell this session how much of the awaited seat's decision clock is
    /// left, as read by whoever owns the clock.
    ///
    /// `seq` is the [`Session::decision_seq`] the reading was taken against,
    /// and passing it is not ceremony: the caller reads its clock *before*
    /// handing a frame in, and handling that frame may move the game, so by
    /// the time a view is built the number can already belong to a question
    /// nobody is being asked any more.
    ///
    /// `remaining_ms` of `None` states that **no decision clock is running** —
    /// an untimed table, an AI chair, or a seat waiting out its reconnect
    /// window rather than deciding. That is a different statement from never
    /// having called this at all, and [`Session::decision_remaining_ms`]
    /// treats them differently.
    pub const fn set_decision_remaining(&mut self, seq: u64, remaining_ms: Option<u32>) {
        self.clock = Some((seq, remaining_ms));
    }

    /// How long the awaited seat has left, in milliseconds, or `None` when no
    /// decision clock is running.
    ///
    /// Almost all of this is the anchor check. A reading is only true of the
    /// question it was taken for, so once [`Session::decision_seq`] has moved
    /// past it the reading is discarded and the seat is given the table's
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
    pub fn decision_remaining_ms(&self) -> Option<u32> {
        if let Some((seq, reading)) = self.clock
            && seq == self.decisions
        {
            return reading;
        }
        let seat = self.awaiting_seat()?;
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
        // Teams included, as `stand_in` builds it: without them every other
        // chair reads as an enemy, the seat's own partner among them.
        let agent = HeuristicAgent::new(AIProfile::default()).with_teams(self.teams.clone());
        let pending = self.engine.pending();
        let view = crate::view::player_view(
            self.engine.state(),
            player,
            self.seq,
            Some(pending),
            &crate::view::SeatContext {
                awaiting: pending_player(pending),
                held: self.engine.automation(player).hold.suppresses(),
                owed: crate::view::owed_payment(&self.engine),
                // As in `pump`, and pointedly so here: this view exists
                // because a clock ran out, so it is the one place the
                // expired number could reach a rules decision.
                decision_remaining_ms: None,
            },
            &self.house_answered,
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
            self.seq,
            Some(&pending),
            &crate::view::SeatContext {
                awaiting: pending_player(&pending),
                held: self.engine.automation(seat).hold.suppresses(),
                owed: crate::view::owed_payment(&self.engine),
                decision_remaining_ms: self.decision_remaining_ms(),
            },
            &self.house_answered,
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
        self.answer(player, action, None)
    }

    /// The decision clock ran out on `seat`: the house answers this one
    /// question for it, and the seat is marked as answered by the clock
    /// until it answers one itself.
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
    ) -> Option<Result<Vec<(PlayerId, Envelope)>, String>> {
        let (player, action) = self.timeout_action()?;
        if player != seat {
            return None;
        }
        Some(self.answer(player, action, Some(HouseAnswer::Clock)))
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
        let by = by.filter(|_| !kind.is_ai_chair());
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
            self.house_answered[player.get() as usize] = by;
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
    /// to, so seat 1 has nothing else to read it off.
    #[test]
    fn a_seat_that_holds_no_priority_is_still_the_seat_being_waited_for() {
        // Both seats human, so the seat that is not being asked is sent a
        // view and the second assertion has something to read.
        let mut preset = test_preset();
        preset.seats[1].controller = SeatController::Open;
        let mut session = Session::new(&preset).expect("session builds");
        let asked = PlayerId::new(0);
        let bystander = PlayerId::new(1);

        assert!(
            matches!(session.pending(), Pending::Mulligan { .. }),
            "the game opens on a question nobody holds priority for"
        );
        assert_eq!(session.awaiting_seat(), Some(asked));
        assert_eq!(
            seat_view(&session, asked).awaiting,
            Some(asked),
            "the seat being asked is told the table is waiting for it"
        );
        assert_eq!(
            seat_view(&session, bystander).awaiting,
            Some(asked),
            "and so is the seat that is not being asked, which has no other \
             way to know"
        );

        // Play the game out with the house agent answering both chairs, and
        // hold every view against the seat that actually owes an answer.
        // Bounded on the questions seen rather than on the loop, because a
        // run that stopped after the mulligans would assert almost nothing.
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
            assert_eq!(
                seat_view(&session, bystander).awaiting,
                Some(seat),
                "every question names the seat that owes the answer"
            );
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

    /// The house that answers for a timed-out seat plays for that seat's
    /// team. It used to be built with no teams, so every other chair looked
    /// like an opponent. The engine never offers a teammate as a defender,
    /// but the agent's own reading of the table still counted one as a
    /// threat. Here the seat is on 5 life beside a teammate with eight hasty
    /// goblins, and the opponents have nothing: a house that fears the swing
    /// back from its own partner keeps its attackers home. `stand_in` built
    /// its agent with the table's teams all along; the clock's did not.
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

        let (player, action) = session.timeout_action().expect("the attack is asked");
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

    /// Answers for every seat but `seat` until `seat` is asked, as the
    /// players at those seats would: with the house's legal answer, through
    /// the socket door, so none of them is marked as answered by the house.
    fn until_asked(session: &mut Session, seat: PlayerId) {
        for _ in 0..200 {
            let asked = session.awaiting_seat().expect("the game goes on");
            if asked == seat {
                return;
            }
            let (player, action) = session.timeout_action().expect("a question is out");
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

        let routed = session
            .answer_by_clock(me)
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
        let (_, action) = session.timeout_action().expect("the seat is asked");
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
        assert!(session.answer_by_clock(PlayerId::new(1)).is_none());
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
        let (_, action) = session.timeout_action().expect("the seat is asked");
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
        let routed = session
            .answer_by_clock(driven)
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
}
