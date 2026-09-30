//! A table in this process: the host's `Session`, with a [`SeatCore`] in
//! every chair, fed the bytes a seat's socket would carry and answering
//! with the actions a socket would send.
//!
//! What it leaves out is the transport: no gateway, no engine process, no
//! curtain and no clock. What it keeps is everything the bridge decides:
//! the same frames in the same order, refusals as the engine answers them
//! (an `Error` and the question again, `EngineRunner::refused`), and every
//! seat's minds asked in one round before any answer is awaited, as seats at
//! one table are.

#![allow(dead_code)]

use baylee_core::ids::PlayerId;
use baylee_core::preset::SeatController;
use baylee_engine::choice::PlayerAction;
use baylee_engine::win::GameResult;
use baylee_gamehost::session::Session;
use baylee_protocol::v1::{self, Envelope};
use baylee_seat::deck::Deck;
use baylee_seat::lobby::seat_name;
use baylee_seat::transcript::Note;
use baylee_seat::{BridgeConfig, Mind, Request, SeatCore, Stats, Step};
use futures_util::future::join_all;
use prost::Message as _;
use std::collections::VecDeque;
use std::fmt::Write as _;
use std::sync::Arc;
use std::time::Duration;

/// A bridge configuration for a table in this process: no margin for a
/// network there is not, and the pacing floor is the driver's (there is
/// none here).
#[must_use]
pub fn config() -> BridgeConfig {
    BridgeConfig {
        think: Duration::from_secs(60),
        margin: Duration::ZERO,
        ..BridgeConfig::default()
    }
}

/// How a game at a [`Table`] went.
#[derive(Debug)]
pub struct Played {
    /// Each seat's counts.
    pub stats: Vec<Stats>,
    /// How it ended, when it did.
    pub result: Option<GameResult>,
    /// Actions the table took.
    pub actions: u32,
    /// Each seat's transcript.
    pub notes: Vec<Vec<Note>>,
}

/// Two decks, two minds, one game.
pub struct Table {
    pub session: Session,
    pub cores: Vec<SeatCore>,
    minds: Vec<Arc<dyn Mind>>,
    inbox: VecDeque<(PlayerId, Envelope)>,
    outbox: VecDeque<(PlayerId, PlayerAction)>,
    asks: Vec<(usize, Box<Request>)>,
    notes: Vec<Vec<Note>>,
    /// Every request any mind was asked, in order.
    pub requests: Vec<(usize, Request)>,
    pub result: Option<GameResult>,
    pub actions: u32,
    /// Rounds in which minds were asked, and how many at once.
    pub rounds: Vec<usize>,
}

impl Table {
    /// A table of `decks`, seat by seat, each played by its mind.
    ///
    /// # Panics
    /// When the decks do not make a playable table.
    #[must_use]
    pub fn new(
        seed: u64,
        decks: [&Deck; 2],
        minds: [Arc<dyn Mind>; 2],
        config: &BridgeConfig,
    ) -> Self {
        let loaded: Vec<_> = decks
            .iter()
            .map(|deck| {
                baylee_cards::decks::from_lines(
                    &deck.name,
                    &deck.cards,
                    &deck.sideboard,
                    &deck.commanders,
                )
                .expect("the deck resolves")
            })
            .collect();
        let mut preset = baylee_cards::decks::preset_for(seed, &loaded[0], &loaded[1]);
        for seat in &mut preset.seats {
            // An open chair is a socket's: the Session asks it over the wire.
            seat.controller = SeatController::Open;
        }
        let mut session = Session::new(&preset).expect("a playable table");
        // Each chair called what its mind says it is, as the bridge signs in.
        let names = ["A", "B"]
            .iter()
            .zip(&minds)
            .map(|(name, mind)| {
                seat_name(mind.disclosure(), name).expect("a name the gateway takes")
            })
            .collect();
        session.describe(format!("selfplay-{seed}"), names);
        let cores = decks
            .iter()
            .zip(&minds)
            .map(|(deck, mind)| SeatCore::new(config.clone(), deck.list.clone(), mind.disclosure()))
            .collect();
        let mut inbox = VecDeque::new();
        for seat in 0..2 {
            let seat = PlayerId::new(seat);
            inbox.push_back((seat, session.game_static_envelope(seat)));
        }
        inbox.extend(session.pump());
        Self {
            session,
            cores,
            minds: minds.into(),
            inbox,
            outbox: VecDeque::new(),
            asks: Vec::new(),
            notes: vec![Vec::new(), Vec::new()],
            requests: Vec::new(),
            result: None,
            actions: 0,
            rounds: Vec::new(),
        }
    }

    /// One step of the table: every frame delivered, then every mind asked
    /// at once, or else one queued answer applied. `false` when nothing is
    /// left to do.
    pub async fn step(&mut self) -> bool {
        self.deliver();
        if !self.asks.is_empty() {
            self.think().await;
            return true;
        }
        let Some((seat, action)) = self.outbox.pop_front() else {
            return false;
        };
        self.actions += 1;
        match self.session.act(seat, action) {
            Ok(routed) => self.inbox.extend(routed),
            Err(reason) => {
                self.inbox.push_back((seat, error(&reason)));
                self.inbox
                    .extend(self.session.reask(seat).into_iter().map(|env| (seat, env)));
            }
        }
        true
    }

    /// What the table is waiting for, and what each seat last wrote: the
    /// message of a game that stopped without an end.
    #[must_use]
    pub fn stall(&self) -> String {
        let mut out = format!("pending {:?}\n", self.session.pending());
        for (seat, notes) in self.notes.iter().enumerate() {
            for note in notes.iter().rev().take(6).rev() {
                let _ = writeln!(
                    out,
                    "  seat {seat}: {}",
                    serde_json::to_string(note).unwrap_or_default()
                );
            }
        }
        out
    }

    /// Plays until the game ends, nothing is left to do, or `max_actions`
    /// actions were taken.
    ///
    /// # Panics
    /// When nothing is left to do and the game has not ended: a question
    /// nobody answers.
    pub async fn play(mut self, max_actions: u32) -> Played {
        while self.result.is_none() && self.actions < max_actions {
            let stepped = self.step().await;
            assert!(
                stepped || self.result.is_some(),
                "the table stopped after {} actions:\n{}",
                self.actions,
                self.stall()
            );
        }
        self.deliver();
        for (seat, core) in self.cores.iter_mut().enumerate() {
            self.notes[seat].extend(core.take_notes());
        }
        Played {
            stats: self.cores.iter().map(|core| core.stats().clone()).collect(),
            result: self.result,
            actions: self.actions,
            notes: self.notes,
        }
    }

    fn deliver(&mut self) {
        while let Some((seat, envelope)) = self.inbox.pop_front() {
            let at = usize::from(seat.get());
            let steps = self.cores[at].hear(&envelope.encode_to_vec());
            self.take(at, steps);
        }
    }

    async fn think(&mut self) {
        let asks = std::mem::take(&mut self.asks);
        self.rounds.push(asks.len());
        let thinking: Vec<_> = asks
            .into_iter()
            .map(|(seat, request)| {
                self.requests.push((seat, (*request).clone()));
                let question = request.question;
                // Called here, before anything is awaited: every mind is asked
                // in this round before any answer is waited for.
                let answer = self.minds[seat].decide(*request);
                async move { (seat, question, answer.await) }
            })
            .collect();
        for (seat, question, result) in join_all(thinking).await {
            let steps = self.cores[seat].answered(question, result);
            self.take(seat, steps);
        }
    }

    fn take(&mut self, seat: usize, steps: Vec<Step>) {
        for step in steps {
            match step {
                // Ready and resume are the transport's, and so is taking a
                // mind off the table: this table has no curtain and never
                // drops a socket.
                Step::Send(_) | Step::MindDown => {}
                Step::Answer { action, .. } => {
                    self.outbox
                        .push_back((PlayerId::new(u8::try_from(seat).unwrap()), action));
                }
                Step::Ask(request) => self.asks.push((seat, request)),
                Step::Cancel(question) => self
                    .asks
                    .retain(|(s, r)| *s != seat || r.question != question),
                Step::Leave(reason) => panic!("seat {seat} left the table: {reason}"),
                Step::Over(result) => self.result = Some(result),
            }
        }
        let notes = self.cores[seat].take_notes();
        self.notes[seat].extend(notes);
    }
}

/// The engine's refusal, as `EngineRunner` sends it.
fn error(message: &str) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::Error(v1::Error {
            code: 1,
            message: message.to_string(),
        })),
    }
}

/// The median and the 90th percentile of `values` (nearest rank).
#[must_use]
pub fn median_p90(values: &[u32]) -> (u32, u32) {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = |p: usize| sorted[(sorted.len() * p).div_ceil(100).saturating_sub(1)];
    (rank(50), rank(90))
}
