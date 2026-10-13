//! A replay: a game record played again on a fresh session, handing on what
//! one perspective is sent at every moment (`docs/protocol.md` §"Replays").
//!
//! Re-exported as `baylee_gamehost::replay`. The record is omniscient; what
//! leaves here is a projection: the frames a socket at one seat (or a
//! spectator) was sent, built by the very functions a live seat's frames come
//! from ([`Session::seat_view`], [`Session::spectator_envelopes`]), after each
//! input applied through the live answer path (`Session::replayed`).
//!
//! **No AI runs**: the house's answers are the record's inputs, and nothing
//! here pumps, so a projection is deterministic.
//!
//! **Checked as it goes**: after every input the engine's hash against the
//! record's ([`Check::Exact`], the same build), else the public table's
//! digest against the record's `seen` ([`Check::Seen`], another build), else
//! the replay is no longer known to be the recorded game: a divergence, which
//! stops it ([`Verify::Stop`]) or is noted and passed ([`Verify::Note`]), the
//! rest [`Check::Unverified`].

use std::ops::ControlFlow;

use super::{SeatKind, Session, choice_envelope};
use crate::record::{self, Line, RECORD_VERSION, hex};
use baylee_core::ids::{CardIndex, PlayerId};
use baylee_core::preset::{SeatController, SeatSpec};
use baylee_engine::choice::Pending;
use baylee_protocol::v1::Envelope;
use baylee_view::{PlayerView, SpectatorView};

pub use crate::record::ReplayError;
pub use baylee_protocol::tape::{Basis, Check, Divergence};

/// Whose view of the game a projection hands on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Perspective {
    /// One seat, exactly as a socket at it would have been told the game.
    Seat(PlayerId),
    /// Nobody in particular: the spectators' stream.
    Public,
    /// One seat with every other hand beside its own. Only for a record its
    /// holder already holds whole (a game this computer hosted against the
    /// house): the caller decides; this crate cannot know where a record
    /// came from.
    AllHands(PlayerId),
}

/// What a check that fails does.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Verify {
    /// Stop there; everything before it stands.
    #[default]
    Stop,
    /// Go on, noting where: the player asked to play on.
    Note,
}

/// The frames the perspective is sent after one record input.
#[derive(Debug)]
pub struct Moment {
    /// The record input (its line number `n`) after which they were made;
    /// `None` before the first.
    pub input: Option<u64>,
    /// The record's host time at that input, milliseconds since the Unix
    /// epoch; the first input's before any.
    pub at: u64,
    /// How far this moment is known to be the recorded game's.
    pub check: Check,
    /// The frames, in the order a socket is sent them. Empty when nothing
    /// visible changed: the view (its `seq` aside) is the last one sent and
    /// the log has no new line.
    pub frames: Vec<Envelope>,
}

/// Why a projection ended where it did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// The record's end, which the replayed game reached too.
    End,
    /// The record ends without an end: its engine died, or it was cut.
    Cut,
    /// A check failed under [`Verify::Stop`] after this input (`None`:
    /// before the first); nothing from it on was handed on.
    Diverged {
        /// The input.
        input: Option<u64>,
    },
    /// This build's engine refused this input.
    Refused {
        /// The input.
        input: u64,
    },
    /// The sink asked to stop.
    Cancelled,
}

/// How far a projection went, and how far it is known to be the record's.
#[derive(Debug, PartialEq, Eq)]
pub struct Projection {
    /// The record inputs applied.
    pub inputs: u64,
    /// The last input whose engine hash was the record's (`None`: none).
    pub exact_to: Option<u64>,
    /// The last input whose `seen` was the record's (`None`: none).
    pub seen_to: Option<u64>,
    /// The first divergence, when one was found.
    pub diverged: Option<Divergence>,
    /// Why it ended.
    pub stop: Stop,
}

/// What the record says the table was after one input, and how the replay
/// compares.
struct Checker {
    verify: Verify,
    exact_to: Option<u64>,
    seen_to: Option<u64>,
    diverged: Option<Divergence>,
}

impl Checker {
    /// Checks the session against what the record holds after `input`. `Err`
    /// is a divergence the caller must stop at.
    fn check(
        &mut self,
        session: &Session,
        input: Option<u64>,
        hash: &str,
        seen: Option<&str>,
    ) -> Result<Check, ()> {
        if self.diverged.is_some() {
            return Ok(Check::Unverified);
        }
        if hex(session.engine.snapshot_hash()) == hash {
            if input.is_some() {
                self.exact_to = input;
                if seen.is_some() {
                    self.seen_to = input;
                }
            }
            return Ok(Check::Exact);
        }
        let replayed = hex(record::seen(session.engine.state()));
        if seen == Some(replayed.as_str()) {
            if input.is_some() {
                self.seen_to = input;
            }
            return Ok(Check::Seen);
        }
        self.diverged = Some(Divergence {
            input,
            by: if seen.is_some() {
                Basis::Seen
            } else {
                Basis::Hash
            },
        });
        match self.verify {
            Verify::Stop => Err(()),
            Verify::Note => Ok(Check::Unverified),
        }
    }

    fn finish(self, inputs: u64, stop: Stop) -> Projection {
        Projection {
            inputs,
            exact_to: self.exact_to,
            seen_to: self.seen_to,
            diverged: self.diverged,
            stop,
        }
    }
}

/// The last view handed on, to tell a moment that changed nothing.
enum Last {
    Nothing,
    Seat(Box<PlayerView>),
    Public(Box<SpectatorView>),
}

/// The session and the perspective it is told from.
struct Teller {
    session: Session,
    perspective: Perspective,
    last: Last,
}

impl Teller {
    /// The frames the perspective is sent now, or none when nothing it can
    /// see changed since the last.
    fn frames(&mut self) -> Vec<Envelope> {
        let session = &mut self.session;
        let over = matches!(session.engine.pending(), Pending::GameOver(_));
        match self.perspective {
            Perspective::Seat(seat) | Perspective::AllHands(seat) => {
                let view = if matches!(self.perspective, Perspective::AllHands(_)) {
                    session.seat_view_showing_all(seat)
                } else {
                    session.seat_view(seat)
                };
                let same = match &self.last {
                    Last::Seat(last) => {
                        !over && !session.log_untold(seat) && same_seat_view(last, &view)
                    }
                    _ => false,
                };
                if same {
                    return Vec::new();
                }
                let mut out = session.seat_envelopes(seat, &view);
                if over {
                    out.push(choice_envelope(session.seq, session.engine.pending()));
                }
                self.last = Last::Seat(Box::new(view));
                out
            }
            Perspective::Public => {
                let view = session.spectator_view();
                let same = match &self.last {
                    Last::Public(last) => {
                        !over
                            && session.gallery.told >= session.log.len()
                            && same_public_view(last, &view)
                    }
                    _ => false,
                };
                if same {
                    return Vec::new();
                }
                let out = session.spectator_envelopes();
                self.last = Last::Public(Box::new(view));
                out
            }
        }
    }

    /// What the perspective is sent before any input: the opening payload,
    /// the view and the log.
    fn opening(&mut self) -> Vec<Envelope> {
        match self.perspective {
            Perspective::Seat(seat) | Perspective::AllHands(seat) => {
                let mut out = vec![self.session.game_static_envelope(seat)];
                out.extend(self.frames());
                out
            }
            Perspective::Public => {
                let out = self.session.spectator_snapshot();
                self.last = Last::Public(Box::new(self.session.spectator_view()));
                out
            }
        }
    }
}

fn same_seat_view(last: &PlayerView, now: &PlayerView) -> bool {
    last.seq == now.seq || {
        let mut now = now.clone();
        now.seq = last.seq;
        *last == now
    }
}

fn same_public_view(last: &SpectatorView, now: &SpectatorView) -> bool {
    last.seq == now.seq || {
        let mut now = now.clone();
        now.seq = last.seq;
        *last == now
    }
}

/// Plays `record` again on a fresh session and hands `sink` what
/// `perspective` is sent at every moment: one before the first input, then
/// one after each input (frames empty where nothing visible changed). `names`
/// name the seats in the opening payload (the record names nobody).
///
/// A record cut off mid-line is played up to the cut ([`Stop::Cut`]).
///
/// # Errors
/// The record does not open with a header this build reads, its preset
/// builds no engine, a complete line does not parse, the perspective names a
/// seat the table does not have ([`ReplayError::NoSuchSeat`]), or the record
/// says the game ended where the replayed one has not. A divergence, a
/// refused input and a cancel are not errors: they are how the projection
/// stopped.
pub fn project(
    record: &[u8],
    names: Vec<String>,
    perspective: Perspective,
    verify: Verify,
    mut sink: impl FnMut(Moment) -> ControlFlow<()>,
) -> Result<Projection, ReplayError> {
    let mut lines = record::lines_of(record).peekable();
    let Some(Line::Header {
        record: RECORD_VERSION,
        preset,
        hash,
        seen,
        ..
    }) = lines.next().transpose()?
    else {
        return Err(ReplayError::NoHeader);
    };
    let mut session = Session::new(&preset).ok_or(ReplayError::Unbuildable)?;
    session.describe("replay".into(), names);
    if let Perspective::Seat(seat) | Perspective::AllHands(seat) = perspective {
        match session.seats.get(seat.get() as usize) {
            None => return Err(ReplayError::NoSuchSeat { seat: seat.get() }),
            // An AI chair is told nothing over a socket; taken over it is
            // told what a socket at it would be, and nothing pumps, so the
            // house plays only what the record says it played.
            Some(SeatKind::Ai(_)) => {
                session.take_over(seat);
            }
            Some(_) => {}
        }
    }
    let mut checker = Checker {
        verify,
        exact_to: None,
        seen_to: None,
        diverged: None,
    };
    let opening = checker.check(&session, None, &hash, seen.as_deref());
    let first_at = match lines.peek() {
        Some(Ok(
            Line::Input { at, .. } | Line::Chair { at, .. } | Line::DeclaredMind { at, .. },
        )) => *at,
        _ => 0,
    };
    session.tell_time(first_at);
    let Ok(check) = opening else {
        return Ok(checker.finish(0, Stop::Diverged { input: None }));
    };
    let mut teller = Teller {
        session,
        perspective,
        last: Last::Nothing,
    };
    let frames = teller.opening();
    if sink(Moment {
        input: None,
        at: first_at,
        check,
        frames,
    })
    .is_break()
    {
        return Ok(checker.finish(0, Stop::Cancelled));
    }
    let mut inputs = 0;
    for line in lines {
        match line? {
            Line::Header { .. } => return Err(ReplayError::NoHeader),
            Line::Chair { .. } | Line::DeclaredMind { .. } => {}
            Line::Input {
                n,
                at,
                seat,
                by,
                action,
                hash,
                seen,
            } => {
                let session = &mut teller.session;
                session.tell_time(at);
                if !session.replayed(PlayerId::new(seat), by, action) {
                    return Ok(checker.finish(inputs, Stop::Refused { input: n }));
                }
                inputs += 1;
                let Ok(check) = checker.check(session, Some(n), &hash, seen.as_deref()) else {
                    return Ok(checker.finish(inputs, Stop::Diverged { input: Some(n) }));
                };
                let frames = teller.frames();
                if sink(Moment {
                    input: Some(n),
                    at,
                    check,
                    frames,
                })
                .is_break()
                {
                    return Ok(checker.finish(inputs, Stop::Cancelled));
                }
            }
            Line::End { n, .. } => {
                if !matches!(teller.session.engine.pending(), Pending::GameOver(_)) {
                    return Err(ReplayError::NotOver { n });
                }
                return Ok(checker.finish(inputs, Stop::End));
            }
        }
    }
    Ok(checker.finish(inputs, Stop::Cut))
}

/// Who played a seat of a local record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Controller {
    /// The player at this computer: the record's open seat.
    You,
    /// The house, at this level (`AIProfile::NAMED`'s key; empty for a
    /// profile that is none of them).
    House(String),
    /// Another player.
    Player,
}

/// One seat of a local record, as the Replays list shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeatSummary {
    /// Who played it.
    pub controller: Controller,
    /// Its team, if the table had teams.
    pub team: Option<u8>,
    /// Its deck's colour identity as `WUBRG` letters (empty: colourless).
    pub colours: String,
    /// Its first commander, for a commander deck.
    pub commander: Option<CardIndex>,
    /// How many cards its deck holds.
    pub cards: usize,
}

/// What the Replays list shows of a local record, read without playing it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Summary {
    /// The build that recorded it.
    pub build: String,
    /// The first line's host time, milliseconds since the Unix epoch (0:
    /// nothing was told).
    pub started: u64,
    /// The last line's.
    pub ended: u64,
    /// The inputs it holds.
    pub inputs: u64,
    /// Whether it has its end.
    pub complete: bool,
    /// The winning seats; empty for a draw or an unfinished game.
    pub winners: Vec<u8>,
    /// Why it ended, as the engine names the reason; empty when it did not.
    pub reason: String,
    /// The turn it ended on, where the record says.
    pub turn: Option<u32>,
    /// The seats in seat order.
    pub seats: Vec<SeatSummary>,
}

/// A deck's colour identity (CR 903.4) as `WUBRG` letters: its cards' and
/// its commanders' together.
#[must_use]
pub fn deck_colours(spec: &SeatSpec) -> String {
    let identity = spec
        .deck
        .iter()
        .chain(&spec.commanders)
        .filter_map(|entry| baylee_cards::by_index(entry.card))
        .fold(baylee_core::color::ColorSet::EMPTY, |set, card| {
            set.union(card.color_identity)
        });
    identity
        .iter()
        .map(baylee_core::color::Color::symbol)
        .collect()
}

/// What the Replays list shows of a local record (`records/`), read by
/// scanning its lines; no engine is built.
///
/// # Errors
/// The record does not open with a header this build reads, or a complete
/// line does not parse.
pub fn summary(record: &[u8]) -> Result<Summary, ReplayError> {
    let mut lines = record::lines_of(record);
    let Some(Line::Header {
        record: RECORD_VERSION,
        build,
        preset,
        ..
    }) = lines.next().transpose()?
    else {
        return Err(ReplayError::NoHeader);
    };
    let mut summary = Summary {
        build,
        started: 0,
        ended: 0,
        inputs: 0,
        complete: false,
        winners: Vec::new(),
        reason: String::new(),
        turn: None,
        seats: preset
            .seats
            .iter()
            .map(|spec| SeatSummary {
                controller: match &spec.controller {
                    SeatController::Open => Controller::You,
                    SeatController::Ai(profile) => Controller::House(
                        baylee_core::preset::AIProfile::NAMED
                            .iter()
                            .find(|(_, named)| named == profile)
                            .map(|(key, _)| (*key).to_owned())
                            .unwrap_or_default(),
                    ),
                    SeatController::Human { .. } => Controller::Player,
                },
                team: spec.team,
                colours: deck_colours(spec),
                commander: spec.commanders.first().map(|entry| entry.card),
                cards: spec.deck.len(),
            })
            .collect(),
    };
    for line in lines {
        let at = match line? {
            Line::Header { .. } => return Err(ReplayError::NoHeader),
            Line::Input { at, .. } => {
                summary.inputs += 1;
                at
            }
            Line::Chair { at, .. } | Line::DeclaredMind { at, .. } => at,
            Line::End {
                at,
                winners,
                reason,
                turn,
                ..
            } => {
                summary.complete = true;
                summary.winners = winners;
                summary.reason = reason;
                summary.turn = turn;
                at
            }
        };
        if summary.started == 0 {
            summary.started = at;
        }
        summary.ended = summary.ended.max(at);
    }
    Ok(summary)
}

#[cfg(test)]
mod smoke {
    use super::*;
    use baylee_protocol::v1;

    fn kind(env: &Envelope) -> &'static str {
        match env.msg {
            Some(v1::envelope::Msg::GameStatic(_)) => "static",
            Some(v1::envelope::Msg::StateDelta(_)) => "delta",
            Some(v1::envelope::Msg::ChoiceRequest(_)) => "choice",
            _ => "other",
        }
    }

    fn run(perspective: Perspective) -> (Projection, Vec<Moment>, u64) {
        let record = crate::record::tests::played(3).take_record();
        let inputs = crate::record::tests::lines(&record)
            .iter()
            .filter_map(|l| match l {
                Line::Input { n, .. } => Some(*n),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut moments = Vec::new();
        let projection = project(
            &record,
            vec!["A".into(), "B".into()],
            perspective,
            Verify::Stop,
            |m| {
                moments.push(m);
                ControlFlow::Continue(())
            },
        )
        .expect("projects");
        assert_eq!(projection.inputs, inputs.len() as u64);
        (projection, moments, *inputs.last().unwrap())
    }

    #[test]
    fn a_whole_record_projects_exactly_for_a_seat_and_the_public() {
        for perspective in [
            Perspective::Seat(PlayerId::new(0)),
            Perspective::Seat(PlayerId::new(1)),
            Perspective::Public,
            Perspective::AllHands(PlayerId::new(0)),
        ] {
            let (projection, moments, last) = run(perspective);
            assert_eq!(projection.stop, Stop::End, "{perspective:?}");
            assert_eq!(projection.exact_to, Some(last));
            assert!(moments.iter().all(|m| m.check == Check::Exact));
            let first: Vec<_> = moments[0].frames.iter().map(kind).collect();
            assert_eq!(first[0], "static", "{perspective:?}: {first:?}");
            assert!(first.contains(&"delta"));
            let shown = moments.iter().filter(|m| !m.frames.is_empty()).count();
            assert!(
                shown > 10 && shown < moments.len(),
                "{perspective:?}: {shown}"
            );
            let end: Vec<_> = moments
                .iter()
                .rev()
                .find(|m| !m.frames.is_empty())
                .unwrap()
                .frames
                .iter()
                .map(kind)
                .collect();
            assert_eq!(
                end.iter().filter(|k| **k == "choice").count(),
                1,
                "{perspective:?}: {end:?}"
            );
            assert_eq!(end.last(), Some(&"choice"));
        }
    }
}
