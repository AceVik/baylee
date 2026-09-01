//! baylee-engine-server — the process a game actually lives in.
//!
//! The gateway routes; it runs no rules. A game runs here, in a process of its
//! own that an agent started and that dialled the gateway back, and the two
//! talk over the engine link described in `docs/protocol.md`.
//!
//! ```text
//!   gateway ──> GameSetup / SeatAttached / SeatFrame ──> EngineRunner
//!   gateway <── SeatFrame / GameEnded ────────────────── EngineRunner
//! ```
//!
//! [`EngineRunner`] is that whole conversation with no socket in it: frames
//! in, frames out, plus a clock the caller is expected to run. Keeping the
//! transport out means the game side can be tested without one, and lets a
//! test drive a real engine over a real link without spawning a process.
//!
//! One process per game is the panic boundary. A rules path that panics takes
//! down exactly one game, and the agent reports the exit — where a gateway
//! hosting sessions in-process had to catch unwinds to get the same effect.

#![warn(missing_docs)]

use baylee_core::ids::PlayerId;
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::Session;
use baylee_protocol::v1::{self, Envelope};

/// What a seat owes, and how long it has to pay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    /// The seat being asked.
    pub seat: PlayerId,
    /// The sequence number it is being asked at. The deadline is anchored to
    /// it, so it restarts when the game actually moves rather than every time
    /// something else happens.
    pub seq: u64,
    /// How long the seat has, in seconds.
    pub secs: u32,
}

/// A game, and everything the gateway can ask of it.
#[derive(Default)]
pub struct EngineRunner {
    /// The game this process was started for.
    game_id: String,
    /// Set by `GameSetup`; every other frame is ignored until then.
    session: Option<Session>,
    /// Which seats have a live socket. The clock only runs for a seat that
    /// can answer.
    attached: Vec<u8>,
    /// Whether `GameEnded` has already been reported.
    ended: bool,
}

impl EngineRunner {
    /// A runner with no game in it yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the game has been built.
    #[must_use]
    pub fn ready(&self) -> bool {
        self.session.is_some()
    }

    /// Whether the game is over and the process may exit.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.ended
    }

    /// The game underneath, for tests and for a harness that wants to look.
    #[must_use]
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// What is on the clock, if anything.
    ///
    /// Nothing is, when the table sets no limit, when nobody is being asked,
    /// or when the seat being asked has no socket to answer on — a player who
    /// walked away is not on a clock they cannot see.
    #[must_use]
    pub fn clock(&self) -> Option<Clock> {
        let session = self.session.as_ref()?;
        let secs = session.decision_timeout_secs();
        if secs == 0 {
            return None;
        }
        let seat = session.awaiting_seat()?;
        if !self.attached.contains(&seat.get()) {
            return None;
        }
        Some(Clock {
            seat,
            seq: session.seq(),
            secs,
        })
    }

    /// The clock ran out. Answers for the seat, and only for that seat at that
    /// sequence number — the opponent may have moved between the timer firing
    /// and this being called, and one seat's expired clock must never take
    /// another seat's decision.
    pub fn timeout(&mut self, clock: Clock) -> Vec<Envelope> {
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        if session.seq() != clock.seq {
            return Vec::new();
        }
        let Some((seat, action)) = session.timeout_action() else {
            return Vec::new();
        };
        if seat != clock.seat {
            return Vec::new();
        }
        self.apply(seat, action)
    }

    /// Applies one frame from the gateway and returns what to send back.
    pub fn handle(&mut self, envelope: Envelope) -> Vec<Envelope> {
        match envelope.msg {
            Some(v1::envelope::Msg::GameSetup(setup)) => self.setup(&setup),
            Some(v1::envelope::Msg::SeatAttached(attached)) => self.attach(&attached),
            Some(v1::envelope::Msg::SeatDetached(detached)) => {
                self.attached.retain(|s| u32::from(*s) != detached.seat);
                Vec::new()
            }
            Some(v1::envelope::Msg::SeatFrame(frame)) => self.seat_frame(&frame),
            _ => Vec::new(),
        }
    }

    /// Builds the game.
    fn setup(&mut self, setup: &v1::GameSetup) -> Vec<Envelope> {
        if self.session.is_some() {
            return Vec::new();
        }
        let Ok(preset) = serde_json::from_slice::<GamePreset>(&setup.preset_json) else {
            return vec![ended(&setup.game_id, "the preset did not decode")];
        };
        let Some(mut session) = Session::new(&preset) else {
            return vec![ended(&setup.game_id, "the preset does not make a game")];
        };
        session.describe(setup.game_id.clone(), setup.seat_names.clone());
        self.game_id.clone_from(&setup.game_id);
        self.session = Some(session);
        Vec::new()
    }

    /// A seat's socket opened. Sends it the payload every later frame refers
    /// to, then either advances the game for it or hands it what it missed.
    fn attach(&mut self, attached: &v1::SeatAttached) -> Vec<Envelope> {
        let Ok(seat) = u8::try_from(attached.seat) else {
            return Vec::new();
        };
        let player = PlayerId::new(seat);
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        if !self.attached.contains(&seat) {
            self.attached.push(seat);
        }
        // The roster and the print table go first: everything after this
        // points into them, and a seat earns printings as it sees cards, so
        // this is not a payload that "cannot have changed".
        let mut out = vec![seat_frame(seat, &session.game_static_envelope(player))];
        if attached.resync {
            for env in session.snapshot(player) {
                out.push(seat_frame(seat, &env));
            }
            return out;
        }
        // The account's remembered answers go in before the first pump, so a
        // question the player never wanted to see is already covered when the
        // opening hand arrives. Setting one is not a game action, so this
        // moves nothing and a reconnect simply restates what the seat has.
        for answer in standing_answers(&attached.standing_json) {
            let _ = session.act(player, answer);
        }
        let routed = session.pump();
        out.extend(self.route(&routed));
        out.extend(self.ending());
        out
    }

    /// Tags routed envelopes for the seats that can actually receive them.
    ///
    /// A seat with no socket is dropped here rather than one hop later at the
    /// gateway, and that is what keeps a seat's own opening payload first on
    /// its wire: the frames another seat's attach produced for a player who
    /// had not arrived yet are gone before they can overtake it. Nothing is
    /// lost by it — every attach pumps, and a pump re-sends the current view
    /// to every seat that is present.
    fn route(&self, routed: &[(PlayerId, Envelope)]) -> Vec<Envelope> {
        routed
            .iter()
            .filter(|(p, _)| self.attached.contains(&p.get()))
            .map(|(p, env)| seat_frame(p.get(), env))
            .collect()
    }

    /// A player-facing frame from one seat.
    fn seat_frame(&mut self, frame: &v1::SeatFrame) -> Vec<Envelope> {
        let Ok(seat) = u8::try_from(frame.seat) else {
            return Vec::new();
        };
        let player = PlayerId::new(seat);
        let Ok(inner) = <Envelope as prost::Message>::decode(&frame.envelope[..]) else {
            return Vec::new();
        };
        match inner.msg {
            Some(v1::envelope::Msg::PlayerAction(action_msg)) => {
                let Ok(action) = serde_json::from_slice::<PlayerAction>(&action_msg.action_json)
                else {
                    return Vec::new();
                };
                self.apply(player, action)
            }
            // Read-only: a seat asking for what it missed must not advance the
            // game, or reconnecting would play an AI seat's turn for it.
            Some(v1::envelope::Msg::Resume(resume)) => {
                let Some(session) = self.session.as_ref() else {
                    return Vec::new();
                };
                session
                    .resume(player, resume.last_seq)
                    .iter()
                    .map(|env| seat_frame(seat, env))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// Applies one action and routes everything it produced.
    fn apply(&mut self, player: PlayerId, action: PlayerAction) -> Vec<Envelope> {
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        // An illegal action is the seat's problem, not the game's: `act`
        // already refused it and nothing moved.
        let Ok(routed) = session.act(player, action) else {
            return Vec::new();
        };
        let mut out = self.route(&routed);
        out.extend(self.ending());
        out
    }

    /// `GameEnded`, once, when the game is over.
    ///
    /// The gateway cannot read a `Pending` — it does not link the engine — so
    /// the end of a game has to be said outright.
    fn ending(&mut self) -> Option<Envelope> {
        if self.ended {
            return None;
        }
        let session = self.session.as_ref()?;
        let Pending::GameOver(result) = session.pending() else {
            return None;
        };
        self.ended = true;
        // A draw has no winner, which is a shorter list rather than a
        // different message.
        let winners = result
            .winner
            .map(|p| u32::from(p.get()))
            .into_iter()
            .collect();
        let reason = format!("{:?}", result.reason);
        Some(Envelope {
            msg: Some(v1::envelope::Msg::GameEnded(v1::GameEnded {
                game_id: self.game_id.clone(),
                winners,
                reason,
            })),
        })
    }
}

/// Wraps a player-facing envelope for the seat it belongs to.
#[must_use]
pub fn seat_frame(seat: u8, envelope: &Envelope) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
            seat: u32::from(seat),
            envelope: prost::Message::encode_to_vec(envelope),
        })),
    }
}

/// A `GameEnded` that says why.
fn ended(game_id: &str, reason: &str) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::GameEnded(v1::GameEnded {
            game_id: game_id.to_string(),
            winners: Vec::new(),
            reason: reason.to_string(),
        })),
    }
}

/// The account's remembered answers, as engine actions.
///
/// A handle the registry does not know is dropped rather than applied: it
/// could never fire, and the gateway already refuses to store one.
///
/// Public because it is one half of a seam: the gateway sends the stored
/// preference and this turns it into the handle the engine keeps its
/// automation under. A wrong handle fails silently — the seat is simply asked
/// a question it believed it had answered for good — so the gateway's own
/// tests read its payload back with exactly this function.
#[must_use]
pub fn standing_answers(json: &[u8]) -> Vec<PlayerAction> {
    use baylee_core::ids::{AbilityRef, CardIndex};
    use baylee_engine::choice::StandingAnswer as Answer;

    serde_json::from_slice::<Vec<baylee_protocol::StandingAnswer>>(json)
        .unwrap_or_default()
        .into_iter()
        .filter(|a| baylee_cards::by_index(CardIndex::new(a.card)).is_some())
        .map(|a| PlayerAction::SetStandingAnswer {
            ability: AbilityRef::new(CardIndex::new(a.card), a.ability),
            answer: Some(if a.yes { Answer::Yes } else { Answer::No }),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The acceptance duel, seat 0 human and seat 1 the house.
    fn duel(timeout_secs: u32) -> GamePreset {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/acceptance-decks.txt"),
        )
        .expect("acceptance deck file");
        let a = baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact");
        let b = baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory");
        let mut preset = baylee_cards::decks::preset_for(7, &a, &b);
        preset.seats[0].controller = baylee_core::preset::SeatController::Open;
        preset.house_rules.decision_timeout_secs = timeout_secs;
        preset
    }

    fn setup(runner: &mut EngineRunner, preset: &GamePreset) -> Vec<Envelope> {
        runner.handle(Envelope {
            msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
                game_id: "g1".to_string(),
                preset_json: serde_json::to_vec(preset).expect("preset serializes"),
                seat_names: vec!["You".to_string(), "House".to_string()],
            })),
        })
    }

    fn attach(runner: &mut EngineRunner, seat: u32) -> Vec<Envelope> {
        runner.handle(Envelope {
            msg: Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
                seat,
                standing_json: b"[]".to_vec(),
                resync: false,
            })),
        })
    }

    /// The `(seat, inner message)` of each frame, which is all a test cares
    /// about — the gateway never looks inside one either.
    fn frames(envelopes: &[Envelope]) -> Vec<(u32, v1::envelope::Msg)> {
        envelopes
            .iter()
            .filter_map(|env| match &env.msg {
                Some(v1::envelope::Msg::SeatFrame(frame)) => {
                    let inner = <Envelope as prost::Message>::decode(&frame.envelope[..]).ok()?;
                    Some((frame.seat, inner.msg?))
                }
                _ => None,
            })
            .collect()
    }

    /// A seat's first frame has to be the roster and the print table: every
    /// frame after it points into them, and without the print table a
    /// `PrintRef` names no card at all.
    #[test]
    fn a_seats_first_frame_is_the_payload_the_rest_refers_to() {
        let mut runner = EngineRunner::new();
        assert!(
            setup(&mut runner, &duel(0)).is_empty(),
            "setup says nothing"
        );
        assert!(runner.ready());
        let out = attach(&mut runner, 0);
        let frames = frames(&out);
        assert!(
            matches!(frames.first(), Some((0, v1::envelope::Msg::GameStatic(_)))),
            "expected the opening payload first, got {:?}",
            frames.first().map(|(seat, _)| seat)
        );
    }

    /// Frames for a seat that has not arrived are dropped here rather than one
    /// hop later, which is what keeps that seat's own opening payload first on
    /// its wire when it does arrive.
    #[test]
    fn a_seat_with_no_socket_is_sent_nothing() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(0));
        let out = attach(&mut runner, 0);
        assert!(
            frames(&out).iter().all(|(seat, _)| *seat == 0),
            "the absent seat was sent something"
        );
        // And once it arrives, its own attach hands it the whole state.
        let out = attach(&mut runner, 1);
        let frames = frames(&out);
        assert!(matches!(
            frames.first(),
            Some((1, v1::envelope::Msg::GameStatic(_)))
        ));
        assert!(
            frames.len() > 1,
            "a seat that arrives late is left with only a roster"
        );
    }

    /// The clock is the one thing the rules kernel must not own, and it must
    /// not run against a player who is not there to see it.
    #[test]
    fn nobody_is_on_a_clock_they_cannot_see() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(60));
        assert_eq!(runner.clock(), None, "no seat has a socket yet");
        attach(&mut runner, 0);
        let clock = runner.clock().expect("the seat being asked is here");
        assert_eq!(clock.seat.get(), 0);
        assert_eq!(clock.secs, 60);
        runner.handle(Envelope {
            msg: Some(v1::envelope::Msg::SeatDetached(v1::SeatDetached {
                seat: 0,
            })),
        });
        assert_eq!(runner.clock(), None, "the player walked away");
    }

    /// A table with no limit puts nobody on a clock at all.
    #[test]
    fn no_limit_means_no_clock() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(0));
        attach(&mut runner, 0);
        assert_eq!(runner.clock(), None);
    }

    /// One seat's expired clock must never take another seat's decision, and
    /// a deadline that fired after the game moved on must take none at all.
    #[test]
    fn a_stale_deadline_answers_for_nobody() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(60));
        attach(&mut runner, 0);
        let clock = runner.clock().expect("someone is being asked");
        let stale = Clock {
            seq: clock.seq + 1,
            ..clock
        };
        assert!(
            runner.timeout(stale).is_empty(),
            "a deadline armed for an older question answered the current one"
        );
        let other_seat = Clock {
            seat: PlayerId::new(1),
            ..clock
        };
        assert!(
            runner.timeout(other_seat).is_empty(),
            "one seat's clock answered for another"
        );
        assert!(
            !runner.timeout(clock).is_empty(),
            "the seat's own deadline did nothing"
        );
    }

    /// A preset that does not describe a game has to say so. The gateway
    /// cannot read a `Pending` and would otherwise hold a table open forever
    /// waiting for a game that was never built.
    #[test]
    fn a_game_that_cannot_start_says_so_rather_than_hanging() {
        let mut runner = EngineRunner::new();
        let out = runner.handle(Envelope {
            msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
                game_id: "g1".to_string(),
                preset_json: b"not a preset".to_vec(),
                seat_names: Vec::new(),
            })),
        });
        assert!(
            matches!(
                out.first().map(|e| &e.msg),
                Some(Some(v1::envelope::Msg::GameEnded(_)))
            ),
            "a broken setup was swallowed"
        );
        assert!(!runner.ready());
    }

    /// Everything before the game exists is ignored rather than acted on: a
    /// seat frame that arrives first is a race, not an attack, and neither
    /// deserves a panic.
    #[test]
    fn frames_before_the_game_exists_do_nothing() {
        let mut runner = EngineRunner::new();
        assert!(attach(&mut runner, 0).is_empty());
        assert!(
            runner
                .handle(Envelope {
                    msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                        seat: 0,
                        envelope: Vec::new(),
                    })),
                })
                .is_empty()
        );
        assert!(!runner.finished());
    }

    /// A second `GameSetup` must not rebuild a game that is already being
    /// played — the seats would silently be handed a different one.
    #[test]
    fn a_game_is_built_once() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(0));
        let seq_before = runner.session().expect("a game").seq();
        assert!(setup(&mut runner, &duel(0)).is_empty());
        assert_eq!(
            runner.session().expect("still the same game").seq(),
            seq_before
        );
    }
}
