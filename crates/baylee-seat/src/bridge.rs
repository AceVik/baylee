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
use baylee_protocol::v1::{Envelope, SeatMind};
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

/// Another mind for the seat, from its next decision on: what a debug
/// bridge is ordered during its game (`docs/llm-seat.md` §"Changing a
/// chair during the game").
pub struct Swap {
    /// The mind that answers from the next question the seat is asked.
    pub mind: Arc<dyn Mind>,
    /// The longest one answer may take from then on, as the new mind's
    /// profile says (`think_secs`); `None` keeps the bridge's.
    pub think: Option<Duration>,
    /// Called once the swap is made, at that question: the old mind has
    /// answered its last (its spend can be settled).
    pub taken: Option<Box<dyn FnOnce() + Send>>,
    /// What the new mind says it is (`docs/protocol.md` §"Who answers a
    /// seat, as it says"): told to the table before the new mind's first
    /// answer, so the game's record writes the change where it happened.
    pub declared: Option<SeatMind>,
}

impl std::fmt::Debug for Swap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Swap")
            .field("mind", &self.mind.disclosure())
            .field("think", &self.think)
            .field("taken", &self.taken.is_some())
            .field("declared", &self.declared.as_ref().map(|m| m.model.as_str()))
            .finish()
    }
}

/// Where [`Swap`]s come from.
pub type Swaps = tokio::sync::mpsc::UnboundedReceiver<Swap>;

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
    play_swapping(link, core, mind, transcript, options, None)
}

/// [`play`], taking another mind whenever `swaps` hands one over: never in
/// the middle of a question, which the mind that was asked it answers, but
/// at the next one the seat asks a mind.
///
/// # Errors
/// As [`play`].
pub fn play_swapping<'a>(
    link: &'a mut SeatLink,
    core: SeatCore,
    mind: Arc<dyn Mind>,
    transcript: &'a mut Transcript,
    options: &'a PlayOptions,
    swaps: Option<Swaps>,
) -> Playing<'a> {
    Box::pin(run(link, core, mind, transcript, options, swaps))
}

async fn run(
    link: &mut SeatLink,
    mut core: SeatCore,
    mind: Arc<dyn Mind>,
    transcript: &mut Transcript,
    options: &PlayOptions,
    swaps: Option<Swaps>,
) -> anyhow::Result<Played> {
    let mut bridge = Bridge {
        mind,
        options: options.clone(),
        thinking: None,
        asked: None,
        swaps,
        rethink: None,
        redeclare: None,
        declaring: Vec::new(),
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
            bridge.settle(&mut core);
            let steps = core.resumed();
            record(&mut core, transcript);
            if let Next::Lost = bridge.carry_out(steps, socket.as_mut()).await {
                socket = None;
            }
            continue;
        };
        let woken = bridge.wait(ws).await;
        let steps = match woken {
            Woken::Frame(Some(Ok(Message::Binary(bytes)))) => {
                bridge.settle(&mut core);
                core.hear(&bytes)
            }
            Woken::Frame(Some(Ok(Message::Close(_)) | Err(_)) | None) => {
                tracing::info!("the seat socket closed");
                socket = None;
                continue;
            }
            Woken::Frame(Some(Ok(_))) => continue,
            Woken::Joined(joined) => match bridge.answered(&mut core, joined) {
                Some(steps) => steps,
                None => continue,
            },
            Woken::Expired => match bridge.expired(&mut core) {
                Some(steps) => steps,
                None => continue,
            },
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
    /// Other minds, handed over during the game.
    swaps: Option<Swaps>,
    /// The think time of a mind taken since the core was last told it.
    rethink: Option<Duration>,
    /// What a mind taken since the core was last told it says it is.
    redeclare: Option<SeatMind>,
    /// The declaration's steps, sent ahead of the next steps carried out.
    declaring: Vec<Step>,
}

impl Bridge {
    /// Takes every mind handed over since the last question, the latest
    /// last: the mind the next question is asked of.
    fn take_swaps(&mut self) {
        let Some(swaps) = self.swaps.as_mut() else {
            return;
        };
        while let Ok(swap) = swaps.try_recv() {
            self.mind = swap.mind;
            self.rethink = swap.think.or(self.rethink);
            if let Some(declared) = swap.declared {
                self.redeclare = Some(declared);
            }
            if let Some(taken) = swap.taken {
                taken();
            }
        }
    }

    /// Before the core is told anything: takes the minds handed over where
    /// no question is being thought about (the old mind has answered its
    /// last), and tells the core the think time of the mind now playing, so
    /// the questions it asks are bounded by it.
    fn settle(&mut self, core: &mut SeatCore) {
        if self.thinking.is_none() {
            self.take_swaps();
        }
        if let Some(think) = self.rethink.take() {
            core.set_think(think);
        }
        if let Some(declared) = self.redeclare.take() {
            self.declaring.extend(core.declare(declared));
        }
    }

    /// The mind answered (or its task ended): the core's steps, `None`
    /// where no question was being thought about.
    fn answered(
        &mut self,
        core: &mut SeatCore,
        joined: Result<Result<Answer, MindError>, tokio::task::JoinError>,
    ) -> Option<Vec<Step>> {
        let thinking = self.thinking.take()?;
        let result = joined
            .unwrap_or_else(|e| Err(MindError::Unavailable(format!("the mind stopped: {e}"))));
        self.settle(core);
        Some(core.answered(thinking.question, result))
    }

    /// The mind ran out of time: the core's steps, `None` where no question
    /// was being thought about.
    fn expired(&mut self, core: &mut SeatCore) -> Option<Vec<Step>> {
        let thinking = self.thinking.take()?;
        thinking.task.abort();
        self.settle(core);
        Some(core.expired(thinking.question))
    }

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
        let declaring = std::mem::take(&mut self.declaring);
        for step in declaring.into_iter().chain(steps) {
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
                Step::Ask(mut request) => {
                    let now = Instant::now();
                    if self
                        .asked
                        .is_none_or(|(asked, _)| asked != request.question)
                    {
                        self.asked = Some((request.question, now));
                    }
                    self.take_swaps();
                    // A mind taken only now was asked under the old think
                    // time: held to its own where that is shorter (the
                    // core is told before the next question).
                    if let Some(think) = self.rethink {
                        request.budget = request.budget.min(think);
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
    async fn wait_for_mind(&mut self, link: &SeatLink) -> bool {
        loop {
            tokio::time::sleep(self.options.mind_poll).await;
            // A mind handed over meanwhile is the one asked.
            self.take_swaps();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mind::{DeckList, Disclosure, Request, Thinking};
    use crate::seat::BridgeConfig;
    use baylee_client_core::test_support::{ViewBuilder, statics};
    use baylee_core::ids::PlayerId;
    use baylee_core::mana::ManaColor;
    use baylee_engine::choice::{Pending, PlayerAction};
    use baylee_protocol::v1;
    use baylee_view::SeatIdentity;
    use std::sync::Mutex;

    const ME: PlayerId = PlayerId::new(0);

    fn frame(msg: v1::envelope::Msg) -> Vec<u8> {
        Envelope { msg: Some(msg) }.encode_to_vec()
    }

    /// A seat at a two-chair table, its own called as its mind must be.
    fn seated() -> SeatCore {
        let mut game = statics(0);
        game.seats = [(ME, "LLM-seat"), (PlayerId::new(1), "Alice")]
            .into_iter()
            .map(|(player, name)| SeatIdentity {
                player,
                display_name: name.into(),
                is_ai: false,
                away: false,
                team: None,
            })
            .collect();
        let mut core = SeatCore::new(
            BridgeConfig::default(),
            DeckList::default(),
            Disclosure::Llm,
        );
        core.hear(&frame(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
            game_id: game.game_id.clone(),
            view_version: game.view_version,
            static_json: serde_json::to_vec(&game).unwrap(),
        })));
        core
    }

    /// The steps of the seat being asked to choose a colour at `seq`: a
    /// question its standing orders never answer, so the mind is asked.
    fn asked(core: &mut SeatCore, seq: u64) -> Vec<Step> {
        let mut view = ViewBuilder::new(2).build();
        view.seq = seq;
        core.hear(&frame(v1::envelope::Msg::StateDelta(v1::StateDelta {
            game_id: "test-game".into(),
            seq,
            view_json: serde_json::to_vec(&view).unwrap(),
            log_json: Vec::new(),
        })));
        let pending = Pending::ChooseColor {
            player: ME,
            options: vec![ManaColor::Red, ManaColor::Green],
        };
        let steps = core.hear(&frame(v1::envelope::Msg::ChoiceRequest(
            v1::ChoiceRequest {
                game_id: "test-game".into(),
                seq,
                pending_json: serde_json::to_vec(&pending).unwrap(),
            },
        )));
        assert!(
            steps.iter().any(|s| matches!(s, Step::Ask(_))),
            "the mind is asked: {steps:?}"
        );
        steps
    }

    /// A mind that answers `colour`, and writes down that it was asked.
    struct Named {
        colour: ManaColor,
        asked: Arc<Mutex<Vec<ManaColor>>>,
    }

    impl Mind for Named {
        fn decide(&self, _request: Request) -> Thinking<'_> {
            self.asked.lock().unwrap().push(self.colour);
            let colour = self.colour;
            Box::pin(async move { Ok(Answer::new(PlayerAction::ChooseColor(colour))) })
        }

        fn disclosure(&self) -> Disclosure {
            Disclosure::Llm
        }
    }

    /// A mind handed over during the game answers from the next question
    /// on, never the question already being thought about; and once it
    /// does, the swap says so (where the old mind's spend is settled).
    #[tokio::test]
    async fn a_swapped_mind_answers_from_the_next_question_on() {
        let asked_of = Arc::new(Mutex::new(Vec::new()));
        let mind = |colour| -> Arc<dyn Mind> {
            Arc::new(Named {
                colour,
                asked: Arc::clone(&asked_of),
            })
        };
        let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
        let mut bridge = Bridge {
            mind: mind(ManaColor::Red),
            options: PlayOptions::default(),
            thinking: None,
            asked: None,
            swaps: Some(swaps),
            rethink: None,
            redeclare: None,
            declaring: Vec::new(),
        };
        let mut core = seated();
        let first = asked(&mut core, 2);
        // Handed over before the question reaches the mind: it is the new
        // mind's, at the moment it is asked.
        let taken = Arc::new(Mutex::new(0));
        let count = Arc::clone(&taken);
        send.send(Swap {
            mind: mind(ManaColor::Green),
            think: None,
            taken: Some(Box::new(move || *count.lock().unwrap() += 1)),
            declared: None,
        })
        .unwrap_or_else(|_| panic!("the bridge listens"));
        bridge.carry_out(first, None).await;
        assert_eq!(*taken.lock().unwrap(), 1);
        let thinking = bridge.thinking.take().unwrap();
        core.answered(thinking.question, thinking.task.await.unwrap());
        assert_eq!(*asked_of.lock().unwrap(), [ManaColor::Green]);
        // One handed over while a question is being thought about waits
        // for the next question.
        let second = asked(&mut core, 3);
        bridge.carry_out(second, None).await;
        send.send(Swap {
            mind: mind(ManaColor::Red),
            think: None,
            taken: None,
            declared: None,
        })
        .unwrap_or_else(|_| panic!("the bridge listens"));
        let thinking = bridge.thinking.take().unwrap();
        core.answered(thinking.question, thinking.task.await.unwrap());
        assert_eq!(
            *asked_of.lock().unwrap(),
            [ManaColor::Green, ManaColor::Green]
        );
        let third = asked(&mut core, 4);
        bridge.carry_out(third, None).await;
        let thinking = bridge.thinking.take().unwrap();
        core.answered(thinking.question, thinking.task.await.unwrap());
        assert_eq!(
            *asked_of.lock().unwrap(),
            [ManaColor::Green, ManaColor::Green, ManaColor::Red]
        );
        assert_eq!(*taken.lock().unwrap(), 1, "called once");
    }

    /// The budget of the question the core asks in `steps`.
    fn budget(steps: &[Step]) -> Option<Duration> {
        steps.iter().find_map(|s| match s {
            Step::Ask(request) => Some(request.budget),
            _ => None,
        })
    }

    /// A mind handed over brings its profile's think time: every question
    /// after it is bounded by that, and the one it is first asked as soon
    /// as it is taken, where that is shorter.
    #[tokio::test]
    async fn a_swapped_mind_thinks_for_as_long_as_its_profile_says() {
        let mind = |colour| -> Arc<dyn Mind> {
            Arc::new(Named {
                colour,
                asked: Arc::new(Mutex::new(Vec::new())),
            })
        };
        let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
        let mut bridge = Bridge {
            mind: mind(ManaColor::Red),
            options: PlayOptions::default(),
            thinking: None,
            asked: None,
            swaps: Some(swaps),
            rethink: None,
            redeclare: None,
            declaring: Vec::new(),
        };
        let mut core = seated();
        let swap = |think| Swap {
            mind: mind(ManaColor::Green),
            think: Some(Duration::from_secs(think)),
            taken: None,
            declared: None,
        };
        send.send(swap(7))
            .unwrap_or_else(|_| panic!("the bridge listens"));
        // Taken before the core hears the question: it asks under 7 s, not
        // the bridge's 60.
        bridge.settle(&mut core);
        let first = asked(&mut core, 2);
        assert_eq!(budget(&first), Some(Duration::from_secs(7)));
        // Taken only as the question reaches the mind: held to 2 s at once.
        send.send(swap(2))
            .unwrap_or_else(|_| panic!("the bridge listens"));
        let before = Instant::now();
        bridge.carry_out(first, None).await;
        let thinking = bridge.thinking.take().unwrap();
        assert!(thinking.deadline <= before + Duration::from_secs(3));
        core.answered(thinking.question, thinking.task.await.unwrap());
        bridge.settle(&mut core);
        let second = asked(&mut core, 3);
        assert_eq!(budget(&second), Some(Duration::from_secs(2)));
    }

    /// A mind swapped during the game says what it is before it answers:
    /// the declaration is carried out ahead of the new mind's first
    /// answer, so the record writes the change where it happened.
    #[tokio::test]
    async fn a_swapped_mind_is_declared_before_its_first_answer() {
        let mind = |colour| -> Arc<dyn Mind> {
            Arc::new(Named {
                colour,
                asked: Arc::new(Mutex::new(Vec::new())),
            })
        };
        let (send, swaps) = tokio::sync::mpsc::unbounded_channel();
        let mut bridge = Bridge {
            mind: mind(ManaColor::Red),
            options: PlayOptions::default(),
            thinking: None,
            asked: None,
            swaps: Some(swaps),
            rethink: None,
            redeclare: None,
            declaring: Vec::new(),
        };
        let mut core = seated();
        let first = asked(&mut core, 2);
        send.send(Swap {
            mind: mind(ManaColor::Green),
            think: None,
            taken: None,
            declared: Some(v1::SeatMind {
                kind: v1::seat_mind::Kind::LlmApi as i32,
                provider: "anthropic".into(),
                model: "claude-sonnet-5-5".into(),
                effort: "high".into(),
                level: String::new(),
            }),
        })
        .unwrap_or_else(|_| panic!("the bridge listens"));
        bridge.carry_out(first, None).await;
        let joined = (&mut bridge.thinking.as_mut().unwrap().task).await;
        let answer = bridge.answered(&mut core, joined).unwrap();
        let declared: Vec<_> = bridge
            .declaring
            .iter()
            .filter_map(|step| match step {
                Step::Send(Envelope {
                    msg: Some(v1::envelope::Msg::SeatMind(mind)),
                }) => Some(mind.model.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(declared, ["claude-sonnet-5-5"]);
        assert!(
            answer.iter().any(|step| matches!(step, Step::Answer { .. })),
            "the new mind's answer follows"
        );
        bridge.carry_out(answer, None).await;
        assert!(bridge.declaring.is_empty(), "sent once, ahead of the answer");
    }

}
