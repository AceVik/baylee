//! A language model watching the neural seat: the seat's own frames, as a
//! socket would carry them, heard by a [`SeatCore`] whose mind is the
//! model. Its answers become BC-only labels and are never played.
//!
//! The neural policy answers every question. The watcher hears every frame
//! the table routes to that seat, and the question again where none was
//! routed before the policy answers it ([`Session::reask`]). Where the seat
//! logic hands a question to the mind, the mind's answer to the question the
//! policy is about to answer is the label. Standing orders, the house's
//! answer for a mind that failed, and anything the referee refused label
//! nothing. The model keeps its conversation across the game, as at a
//! table; what the seat actually did reaches it through the log.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use anyhow::{Context as _, ensure};
use baylee_cards::decks::LoadedDeck;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::PlayerAction;
use baylee_gamehost::session::Session;
use baylee_protocol::v1::Envelope;
use baylee_protocol::v1::envelope::Msg;
use baylee_seat::config::{Overrides, Paths, Plan};
use baylee_seat::mind::MindError;
use baylee_seat::transcript::Note;
use baylee_seat::{BridgeConfig, By, Mind, SeatCore, Step};
use prost::Message as _;
use serde_json::{Value, json};

/// A game's mind, its own log going to the directory given.
type Make = dyn Fn(PathBuf) -> anyhow::Result<Arc<dyn Mind>> + Send + Sync;

/// The model every game's watcher plays, from a seat settings file.
pub(crate) struct Teacher {
    make: Box<Make>,
    /// The longest one answer may take.
    think: Duration,
    identity: Value,
    runtime: tokio::runtime::Runtime,
    /// Set once the model says a limit: no further game asks it.
    exhausted: AtomicBool,
}

impl Teacher {
    /// The profile `name` of the seat settings file `profiles`, checked as
    /// far as it can be before a game asks it: its key or program found.
    pub(in crate::shadow) fn load(profiles: &Path, name: &str) -> anyhow::Result<Self> {
        let env = |key: &str| std::env::var(key).ok();
        let paths = Paths::resolve(Some(profiles), None, &env);
        let file = paths.load().map_err(anyhow::Error::msg)?;
        let plan: Plan = baylee_seat::config::plan(
            None,
            file.as_ref(),
            &paths,
            Some(name),
            &Overrides::default(),
        )
        .map_err(anyhow::Error::msg)?
        .context("The profile names no model")?;
        baylee_seat::llm::check(&plan, &env).map_err(anyhow::Error::msg)?;
        let identity = json!({"profile": plan.profile, "provider": plan.spec.provider.name(),
            "model": plan.spec.model, "effort": plan.settings.effort});
        let think = Duration::from_secs(plan.think_secs);
        let make = move |transcripts: PathBuf| -> anyhow::Result<Arc<dyn Mind>> {
            let mut plan = plan.clone();
            plan.settings.transcripts = Some(transcripts);
            let env = |key: &str| std::env::var(key).ok();
            Ok(baylee_seat::llm::build(&plan, &env)
                .map_err(anyhow::Error::msg)?
                .mind)
        };
        Self::new(Box::new(make), think, identity)
    }

    /// A teacher whose games' minds `make` makes.
    pub(in crate::shadow) fn new(
        make: Box<Make>,
        think: Duration,
        identity: Value,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            make,
            think,
            identity,
            runtime: tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()?,
            exhausted: AtomicBool::new(false),
        })
    }

    /// What labels name as their teacher: the profile, its model and effort.
    pub(in crate::shadow) fn identity(&self) -> Value {
        self.identity.clone()
    }

    /// Whether the model said a limit in any game.
    pub(in crate::shadow) fn exhausted(&self) -> bool {
        self.exhausted.load(Ordering::SeqCst)
    }

    /// A watcher for `seat` of `session`, told the game's static frame;
    /// the mind's own log goes to `transcripts`.
    pub(in crate::shadow) fn watch(
        self: &Arc<Self>,
        session: &mut Session,
        seat: PlayerId,
        deck: &LoadedDeck,
        transcripts: PathBuf,
    ) -> anyhow::Result<Watcher> {
        let mind = (self.make)(transcripts)?;
        let config = BridgeConfig {
            think: self.think,
            margin: Duration::ZERO,
            ..BridgeConfig::default()
        };
        let core = SeatCore::new(config, baylee_seat::deck::list_of(deck), mind.disclosure());
        let mut watcher = Watcher {
            teacher: Arc::clone(self),
            core,
            mind,
            seat,
            answer: None,
            heard_seq: None,
            game: None,
            mind_failures: 0,
        };
        watcher.hear([session.game_static_envelope(seat)]);
        Ok(watcher)
    }
}

/// One game's watcher of one seat.
pub(in crate::shadow) struct Watcher {
    teacher: Arc<Teacher>,
    core: SeatCore,
    mind: Arc<dyn Mind>,
    seat: PlayerId,
    /// The seat logic's last answer: its question, the action and who made it.
    answer: Option<(u64, PlayerAction, By)>,
    /// The frame sequence the last question heard was asked at.
    heard_seq: Option<u64>,
    /// The game and seat the mind played, to release its session.
    game: Option<String>,
    /// Askings that ended in no answer of the mind's.
    pub(in crate::shadow) mind_failures: u64,
}

impl Watcher {
    /// The seat it watches.
    pub(in crate::shadow) const fn seat(&self) -> PlayerId {
        self.seat
    }

    /// Hears frames routed to this seat, asking the mind where its seat
    /// logic says so.
    pub(in crate::shadow) fn hear(&mut self, envelopes: impl IntoIterator<Item = Envelope>) {
        for mut envelope in envelopes {
            match &mut envelope.msg {
                Some(Msg::ChoiceRequest(choice)) => self.heard_seq = Some(choice.seq),
                Some(Msg::GameStatic(statics)) => self.disclose(statics),
                _ => {}
            }
            let steps = self.core.hear(&envelope.encode_to_vec());
            self.take(steps);
        }
    }

    /// The mind's answer to the question `session` now asks this seat at
    /// frame `seq` (its view's), if the mind answered it: hears the
    /// question first where no frame carried it.
    pub(in crate::shadow) fn answer_now(
        &mut self,
        session: &Session,
        seq: u64,
    ) -> Option<(u64, PlayerAction)> {
        if self.heard_seq != Some(seq) {
            self.answer = None;
            self.hear(session.reask(self.seat));
        }
        match self.answer.take() {
            Some((question, action, By::Mind)) => Some((question, action)),
            _ => None,
        }
    }

    /// Names the watched chair in the table's static frame as a chair this
    /// mind may sit in (`LLM-…`): the seat logic leaves a chair called
    /// anything else. The game's own roster, and so its record, keep the
    /// names the evaluator gave them; only the watcher hears this one, and
    /// no chair's name reaches the model.
    fn disclose(&self, statics: &mut baylee_protocol::v1::GameStaticMsg) {
        let Ok(mut table) = serde_json::from_slice::<Value>(&statics.static_json) else {
            return;
        };
        let seat = usize::from(self.seat.get());
        if let Some(chair) = table["seats"].get_mut(seat) {
            chair["display_name"] = json!(format!("{}watcher", self.mind.disclosure().prefix()));
            if let Ok(bytes) = serde_json::to_vec(&table) {
                statics.static_json = bytes;
            }
        }
    }

    /// The seat logic's notes so far.
    pub(in crate::shadow) fn notes(&mut self) -> Vec<Note> {
        self.core.take_notes()
    }

    /// Releases the mind's session for this game.
    pub(in crate::shadow) fn end(&self) {
        if let Some(game) = &self.game {
            self.mind.end_game(game, self.seat.get());
        }
    }

    fn take(&mut self, steps: Vec<Step>) {
        let mut queue: std::collections::VecDeque<Step> = steps.into();
        while let Some(step) = queue.pop_front() {
            match step {
                Step::Answer {
                    question,
                    action,
                    by,
                    ..
                } => self.answer = Some((question, action, by)),
                Step::Ask(request) => {
                    let question = request.question;
                    self.game = Some(request.context.game_id.clone());
                    if self.teacher.exhausted() {
                        let steps = self.core.answered(
                            question,
                            Err(MindError::Unavailable("the teacher is spent".into())),
                        );
                        queue.extend(steps);
                        continue;
                    }
                    let budget = request.budget;
                    let mind = Arc::clone(&self.mind);
                    let result = self.teacher.runtime.block_on(async move {
                        tokio::time::timeout(budget, mind.decide(*request)).await
                    });
                    let steps = if let Ok(result) = result {
                        if let Err(error) = &result {
                            self.mind_failures += 1;
                            if matches!(error, MindError::Limited { .. }) {
                                self.teacher.exhausted.store(true, Ordering::SeqCst);
                            }
                        }
                        self.core.answered(question, result)
                    } else {
                        self.mind_failures += 1;
                        self.core.expired(question)
                    };
                    queue.extend(steps);
                }
                Step::Send(_)
                | Step::Cancel(_)
                | Step::MindDown
                | Step::Leave(_)
                | Step::Over(_) => {}
            }
        }
    }
}

/// The contract a label's header names for an LLM watcher.
pub(in crate::shadow) fn contract(profile: &str, identity: &Value) -> Value {
    json!({"schema": 1, "teacher": format!("llm:{profile}"), "teacher_identity": identity,
        "label_use": "bc_only", "history_mode": "seat_conversation", "scouting": false,
        "teacher_has_behavior_probability": false, "ppo_certification": false,
        "completion_required": true, "path": "shadow/*.jsonl",
        "transcript_path": "shadow/*.seat.jsonl"})
}

/// The watcher's seat notes, one JSON line each, for the label sidecar.
pub(in crate::shadow) fn notes_jsonl(notes: &[Note]) -> anyhow::Result<Vec<u8>> {
    let mut out = Vec::new();
    for note in notes {
        serde_json::to_writer(&mut out, note)?;
        out.push(b'\n');
    }
    ensure!(
        out.len() <= 64 * 1024 * 1024,
        "Watcher transcript exceeds limit"
    );
    Ok(out)
}
