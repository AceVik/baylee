//! One seat played by the net over any table link ([`Table`]: Central's
//! relay for a ranked match, a gateway room's ticketed socket for a
//! `seat4 --pilot` room), to the game's end or to a hand-over.
//!
//! The net's recurrent memory is the game it saw. A socket that drops, a
//! frame it cannot take or an observation that fails ends the flight with
//! [`Flown::HandOver`]: dialling again would show it the latest view, which
//! does not give back what it missed. Whoever flies the seat after it (the
//! ranked runner's house fallback) dials anew on the same link.
//!
//! A real decision is not answered sooner than [`Pace::min_think`] after the
//! frame that completed its question, so a person across the table can
//! follow; a pass, a step of paying mana and a forced answer go at once
//! ([`LiveSeat4::paced`]).

use std::time::Duration;

use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::envelope::Msg;
use baylee_seat::link::{Socket, Table};
use baylee_seat::transcript::{Event, Note};
use baylee_seat::wake::{Standing, Why};
use baylee_seat::{By, Transcript};
use baylee_train::live4::{Asked4, Brain4, By4, LiveSeat4};
use futures_util::{SinkExt as _, StreamExt as _};
use prost::Message as _;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;

/// Why a flight over a socket that closed or failed cannot go on.
pub const SOCKET_LOST: &str = "the seat's socket closed mid-game; the net's recurrent memory cannot be rebuilt from a snapshot";

/// How the seat paces itself.
#[derive(Clone, Copy, Debug)]
pub struct Pace {
    /// The least time between a real decision's question and its answer.
    pub min_think: Duration,
    /// How long the socket may be quiet before the link is asked whether
    /// the game is still on.
    pub room_check: Duration,
}

impl Default for Pace {
    fn default() -> Self {
        let play = baylee_seat::bridge::PlayOptions::default();
        Self {
            min_think: play.min_think,
            room_check: play.room_check,
        }
    }
}

/// How a flight ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Flown {
    /// The table said the game is over ([`LiveSeat4::game_result`]).
    Ended,
    /// The game is gone without a word to this seat: it was over before the
    /// seat sat down, or its room closed.
    Closed,
    /// The net cannot go on, and why, in a sentence that carries no
    /// credential (never a socket's error, whose address may hold a ticket).
    HandOver(String),
}

/// Plays `core`'s seat over `link` until the game ends or the net cannot
/// go on, kept in `transcript` in the bridge's own notes
/// (`baylee_seat::transcript`): each question the net was asked and its
/// answer, by the mind (the policy, raw or under its tactical layer) or the
/// house (the runtime's own fallback), with the time the net took; a forced
/// answer as the standing orders' "nothing to do"; and the game's end, the
/// room's close or the seat's leaving. The socket is opened here and closed
/// before it returns.
///
/// # Errors
/// Only when the link cannot open a socket at all; once the seat holds one,
/// every way the net cannot go on is a [`Flown::HandOver`].
pub async fn fly<L: Table, B: Brain4>(
    link: &mut L,
    core: &mut LiveSeat4<B>,
    pace: Pace,
    transcript: &mut Transcript,
) -> anyhow::Result<Flown> {
    let Some(mut socket) = link.connect().await? else {
        transcript.write(&ended(core, Event::Closed));
        transcript.flush();
        return Ok(Flown::Closed);
    };
    let flown = over(&mut socket, link, core, pace, |note| {
        transcript.write(&note);
    })
    .await;
    let _ = socket.close(None).await;
    match &flown {
        Flown::Ended => {}
        Flown::Closed => transcript.write(&ended(core, Event::Closed)),
        Flown::HandOver(why) => transcript.write(&ended(
            core,
            Event::Left {
                reason: why.clone(),
            },
        )),
    }
    transcript.flush();
    Ok(flown)
}

/// The note on the question the answer last generated was made for.
fn asked(asked: &Asked4) -> Note {
    Note {
        question: asked.question,
        seq: asked.seq,
        turn: asked.turn,
        event: Event::Asked {
            kind: asked.kind,
            // The net has no standing orders: every question is its own.
            why: Some(Why::Question),
            step: asked.step,
            their_turn: asked.their_turn,
            continuing: false,
            retry: false,
            // The net keeps no budget of its own; the table's clock runs.
            budget_ms: 0,
            log_lines: 0,
            controlled: asked.controlled,
        },
    }
}

/// The note on the answer last generated, which took the net `took`.
fn answered<B: Brain4>(
    core: &LiveSeat4<B>,
    asked: &Asked4,
    action: PlayerAction,
    took: Duration,
) -> Note {
    let event = match core.last_by() {
        By4::Forced => Event::Standing {
            kind: asked.kind,
            why: Standing::NothingToDo,
            action,
        },
        by => Event::Answered {
            by: if by == By4::Runtime {
                By::House
            } else {
                By::Mind
            },
            action,
            model_ms: Some(u64::try_from(took.as_millis()).unwrap_or(u64::MAX)),
            note: None,
            thinking: None,
        },
    };
    Note {
        question: asked.question,
        seq: asked.seq,
        turn: asked.turn,
        event,
    }
}

/// The note on how the seat's game ended.
fn ended<B: Brain4>(core: &LiveSeat4<B>, event: Event) -> Note {
    Note {
        question: 0,
        seq: core.last_asked().map_or(0, |asked| asked.seq),
        turn: core.turn(),
        event,
    }
}

async fn over<L: Table, B: Brain4>(
    socket: &mut Socket,
    link: &mut L,
    core: &mut LiveSeat4<B>,
    pace: Pace,
    mut note: impl FnMut(Note),
) -> Flown {
    loop {
        let next = match tokio::time::timeout(pace.room_check, socket.next()).await {
            Ok(next) => next,
            Err(_) if link.room_closed().await => return Flown::Closed,
            Err(_) => continue,
        };
        match next {
            Some(Ok(Message::Binary(bytes))) => {
                let heard = Instant::now();
                let Ok(outgoing) = core.hear(&bytes) else {
                    return Flown::HandOver(stopped(core));
                };
                let took = heard.elapsed();
                for envelope in outgoing {
                    let action = match &envelope.msg {
                        Some(Msg::PlayerAction(action)) => {
                            baylee_core::wire::decode::<PlayerAction>(&action.action).ok()
                        }
                        _ => None,
                    };
                    let question = action.as_ref().and(core.last_asked().copied());
                    if let Some(question) = &question
                        && core.last_by() != By4::Forced
                    {
                        note(asked(question));
                    }
                    if action.is_some() && core.paced() {
                        tokio::time::sleep_until(heard + pace.min_think).await;
                    }
                    let frame = Message::Binary(envelope.encode_to_vec().into());
                    if socket.send(frame).await.is_err() {
                        core.disconnected();
                        return Flown::HandOver(SOCKET_LOST.to_owned());
                    }
                    if let (Some(action), Some(question)) = (action, question) {
                        note(answered(core, &question, action, took));
                    }
                }
                if core.finished() {
                    let result = core
                        .game_result()
                        .map_or_else(String::new, |result| format!("{result:?}"));
                    note(ended(core, Event::Over { result }));
                    return Flown::Ended;
                }
            }
            Some(Ok(Message::Ping(bytes))) => {
                if socket.send(Message::Pong(bytes)).await.is_err() {
                    core.disconnected();
                    return Flown::HandOver(SOCKET_LOST.to_owned());
                }
            }
            Some(Ok(Message::Close(_)) | Err(_)) | None => {
                core.disconnected();
                return Flown::HandOver(SOCKET_LOST.to_owned());
            }
            Some(Ok(_)) => {}
        }
    }
}

/// Why the adapter closed: its own fail-closed reason, which names the
/// frame, the observation or the answer it could not take and nothing of
/// the transport.
fn stopped<B: Brain4>(core: &LiveSeat4<B>) -> String {
    let why = core
        .report()
        .failure
        .as_deref()
        .unwrap_or("the net's adapter stopped");
    format!("the net cannot go on: {why}")
}
