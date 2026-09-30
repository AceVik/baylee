//! The bridge: a [`SeatCore`] on a real socket, a real mind and a real
//! clock.
//!
//! Everything the seat decides is decided in [`SeatCore`]; this module only
//! carries it out. It reads frames and hands them over, runs the mind's
//! answer as a task with a deadline, sends what is to be sent, and owns the
//! three things a core has no way to own:
//!
//! - **time**: a question's budget runs out here ([`SeatCore::expired`]), and
//!   a woken decision is not answered sooner than [`PlayOptions::min_think`]
//!   after it was asked, so a person across the table can follow the game;
//! - **the socket**: a dropped one is dialled again ([`SeatLink`]) and the
//!   table asked for what the seat missed ([`SeatCore::resumed`]);
//! - **a mind that is down**: the socket is left, so the table sees an
//!   absent player and its house holds the chair, and the mind is asked
//!   every [`PlayOptions::mind_poll`] whether it is ready again;
//! - **a room that closed**: a table that could not start, or whose engine
//!   was lost, ends without a word on the seat's socket, which stays open;
//!   the room only leaves the lobby. So the lobby is asked whenever the
//!   socket has been quiet for [`PlayOptions::room_check`], before the seat
//!   dials again, and at every poll of a mind that is down
//!   ([`SeatLink::room_closed`]). A seat that finds its room gone ends its
//!   game with no result ([`SeatCore::closed`]).
//!
//! An answer the core gives while there is no socket is dropped: the table
//! asks the question again on the next socket, at the same `seq`, and the
//! core sends the same answer then.

use crate::link::{SeatLink, Socket};
use crate::mind::{Answer, Mind, MindError};
use crate::seat::{SeatCore, Stats, Step};
use crate::transcript::Transcript;
use baylee_engine::win::GameResult;
use baylee_protocol::v1::Envelope;
use futures_util::{SinkExt as _, StreamExt as _};
use prost::Message as _;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;

/// How the bridge paces itself.
#[derive(Clone, Debug)]
pub struct PlayOptions {
    /// The least time between a woken question and its answer.
    pub min_think: Duration,
    /// How often a mind that is down is asked whether it is ready.
    pub mind_poll: Duration,
    /// How long the socket may be quiet before the lobby is asked whether
    /// the room is still open.
    pub room_check: Duration,
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            min_think: Duration::from_millis(1500),
            mind_poll: Duration::from_secs(5),
            room_check: Duration::from_secs(20),
        }
    }
}

/// How a seat's game went.
#[derive(Debug)]
pub struct Played {
    /// The counts.
    pub stats: Stats,
    /// How the game ended; `None` when the room closed without showing
    /// the seat ([`SeatCore::closed`]).
    pub result: Option<GameResult>,
    /// Sockets opened, the first included.
    pub dials: u32,
}

/// A question the mind is working on.
struct Thinking {
    question: u64,
    deadline: Instant,
    task: JoinHandle<Result<Answer, MindError>>,
}

/// What woke the loop.
enum Woken {
    Frame(Option<Result<Message, tokio_tungstenite::tungstenite::Error>>),
    Joined(Result<Result<Answer, MindError>, tokio::task::JoinError>),
    Expired,
    Quiet,
}

/// What a batch of steps came to.
enum Next {
    Go,
    Lost,
    Down,
    Over(GameResult),
    Left(String),
}

/// A game being played, boxed: it holds the seat's whole memory of the
/// table, too large a future for a caller's stack.
pub type Playing<'a> = Pin<Box<dyn Future<Output = anyhow::Result<Played>> + Send + 'a>>;

/// Plays the seat's game to its end.
///
/// # Errors
/// When the seat leaves the table (a chair not called as the mind must be,
/// a table too fast for it, a version it cannot read), or the table cannot
/// be reached again.
pub fn play<'a>(
    link: &'a mut SeatLink,
    core: SeatCore,
    mind: Arc<dyn Mind>,
    transcript: &'a mut Transcript,
    options: &'a PlayOptions,
) -> Playing<'a> {
    Box::pin(run(link, core, mind, transcript, options))
}

async fn run(
    link: &mut SeatLink,
    mut core: SeatCore,
    mind: Arc<dyn Mind>,
    transcript: &mut Transcript,
    options: &PlayOptions,
) -> anyhow::Result<Played> {
    let mut bridge = Bridge {
        mind,
        options: options.clone(),
        thinking: None,
        asked: None,
    };
    let mut socket = link.connect().await?;
    if socket.is_none() {
        return Ok(bridge.closed(&mut core, transcript, link).await);
    }
    loop {
        let Some(ws) = socket.as_mut() else {
            socket = if link.room_closed().await {
                None
            } else {
                link.connect().await?
            };
            if socket.is_none() {
                return Ok(bridge.closed(&mut core, transcript, link).await);
            }
            let steps = core.resumed();
            record(&mut core, transcript);
            if let Next::Lost = bridge.carry_out(steps, socket.as_mut()).await {
                socket = None;
            }
            continue;
        };
        let woken = bridge.wait(ws).await;
        let steps = match woken {
            Woken::Frame(Some(Ok(Message::Binary(bytes)))) => core.hear(&bytes),
            Woken::Frame(Some(Ok(Message::Close(_)) | Err(_)) | None) => {
                tracing::info!("the seat socket closed");
                socket = None;
                continue;
            }
            Woken::Frame(Some(Ok(_))) => continue,
            Woken::Joined(joined) => {
                let Some(thinking) = bridge.thinking.take() else {
                    continue;
                };
                let result = joined.unwrap_or_else(|e| {
                    Err(MindError::Unavailable(format!("the mind stopped: {e}")))
                });
                core.answered(thinking.question, result)
            }
            Woken::Expired => {
                let Some(thinking) = bridge.thinking.take() else {
                    continue;
                };
                thinking.task.abort();
                core.expired(thinking.question)
            }
            Woken::Quiet => {
                if !link.room_closed().await {
                    continue;
                }
                if let Some(mut ws) = socket.take() {
                    let _ = ws.close(None).await;
                }
                return Ok(bridge.closed(&mut core, transcript, link).await);
            }
        };
        record(&mut core, transcript);
        match bridge.carry_out(steps, socket.as_mut()).await {
            Next::Go => {}
            Next::Lost => socket = None,
            Next::Down => {
                if let Some(mut ws) = socket.take() {
                    let _ = ws.close(None).await;
                }
                if !bridge.wait_for_mind(link).await {
                    return Ok(bridge.closed(&mut core, transcript, link).await);
                }
            }
            Next::Over(result) => {
                if let Some(mut ws) = socket.take() {
                    let _ = ws.close(None).await;
                }
                transcript.flush();
                return Ok(Played {
                    stats: core.stats().clone(),
                    result: Some(result),
                    dials: link.dials(),
                });
            }
            Next::Left(reason) => {
                if let Some(mut ws) = socket.take() {
                    let _ = ws.close(None).await;
                }
                transcript.flush();
                anyhow::bail!("the seat left the table: {reason}");
            }
        }
    }
}

/// Writes what the core noted.
fn record(core: &mut SeatCore, transcript: &mut Transcript) {
    for note in core.take_notes() {
        transcript.write(&note);
    }
}

struct Bridge {
    mind: Arc<dyn Mind>,
    options: PlayOptions,
    thinking: Option<Thinking>,
    /// The question last handed to the mind, and when it was first asked.
    asked: Option<(u64, Instant)>,
}

impl Bridge {
    /// Waits for a frame, the mind's answer, the mind's deadline, or a
    /// quiet spell long enough to ask the lobby about the room.
    async fn wait(&mut self, ws: &mut Socket) -> Woken {
        let deadline = self.thinking.as_ref().map(|t| t.deadline);
        let task = self.thinking.as_mut().map(|t| &mut t.task);
        tokio::select! {
            frame = ws.next() => Woken::Frame(frame),
            joined = async {
                match task {
                    Some(task) => task.await,
                    None => std::future::pending().await,
                }
            } => Woken::Joined(joined),
            () = async {
                match deadline {
                    Some(deadline) => tokio::time::sleep_until(deadline).await,
                    None => std::future::pending().await,
                }
            } => Woken::Expired,
            () = tokio::time::sleep(self.options.room_check) => Woken::Quiet,
        }
    }

    /// The room closed without a result: whatever the mind is thinking
    /// about is dropped, and the seat's game is over.
    async fn closed(
        &mut self,
        core: &mut SeatCore,
        transcript: &mut Transcript,
        link: &SeatLink,
    ) -> Played {
        tracing::info!("the room closed without a result for this seat");
        let steps = core.closed();
        self.carry_out(steps, None).await;
        record(core, transcript);
        transcript.flush();
        Played {
            stats: core.stats().clone(),
            result: None,
            dials: link.dials(),
        }
    }

    /// Carries out the core's steps, in order.
    async fn carry_out(&mut self, steps: Vec<Step>, mut ws: Option<&mut Socket>) -> Next {
        let mut next = Next::Go;
        for step in steps {
            match step {
                Step::Send(envelope) => {
                    if !send(ws.as_deref_mut(), &envelope).await {
                        next = Next::Lost;
                    }
                }
                Step::Answer {
                    question,
                    paced,
                    envelope,
                    ..
                } => {
                    if paced {
                        let since = self
                            .asked
                            .filter(|(asked, _)| *asked == question)
                            .map_or_else(Instant::now, |(_, at)| at);
                        tokio::time::sleep_until(since + self.options.min_think).await;
                    }
                    if !send(ws.as_deref_mut(), &envelope).await {
                        next = Next::Lost;
                    }
                }
                Step::Ask(request) => {
                    let now = Instant::now();
                    if self
                        .asked
                        .is_none_or(|(asked, _)| asked != request.question)
                    {
                        self.asked = Some((request.question, now));
                    }
                    let question = request.question;
                    let deadline = now + request.budget;
                    let mind = Arc::clone(&self.mind);
                    let task = tokio::spawn(async move { mind.decide(*request).await });
                    if let Some(old) = self.thinking.replace(Thinking {
                        question,
                        deadline,
                        task,
                    }) {
                        old.task.abort();
                    }
                }
                Step::Cancel(question) => {
                    if let Some(thinking) = self.thinking.take_if(|t| t.question == question) {
                        thinking.task.abort();
                    }
                }
                Step::MindDown => next = Next::Down,
                Step::Leave(reason) => return Next::Left(reason),
                Step::Over(result) => return Next::Over(result),
            }
        }
        next
    }

    /// Asks a mind that is down whether it is ready, until it is (`true`)
    /// or the room has closed meanwhile (`false`): the house plays on for a
    /// seat whose socket is gone, and may finish the game before the mind
    /// comes back, if it ever does.
    async fn wait_for_mind(&self, link: &SeatLink) -> bool {
        loop {
            tokio::time::sleep(self.options.mind_poll).await;
            if self.mind.ready().await {
                tracing::info!("the mind is ready again");
                return true;
            }
            if link.room_closed().await {
                return false;
            }
        }
    }
}

/// Sends one envelope; `false` when there is no socket or it failed.
async fn send(ws: Option<&mut Socket>, envelope: &Envelope) -> bool {
    let Some(ws) = ws else {
        return false;
    };
    ws.send(Message::Binary(envelope.encode_to_vec().into()))
        .await
        .is_ok()
}
