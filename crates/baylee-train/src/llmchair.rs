//! A language model answering some of a chair's questions in the arena
//! (`arena --llm`), through the seat bridge's mind
//! ([`baylee_seat::llm::ApiMind`]), so a model is measured with the prompt,
//! tools and answer checks it plays with at a real table.
//!
//! The chair is the house's otherwise. The session's own agent at the net's
//! profile, with its seeded noise, keeps playing the seat, and only the
//! questions of the chosen kinds are taken over and handed to the model.
//! With every kind chosen the model plays every question that has more than
//! one answer. A game is therefore the house's baseline game of its deal,
//! move for move, until the first question the model answers. The
//! difference in result against that baseline is the model's doing.
//!
//! The model is handed the seat's view and the question, as a house agent
//! is, and not the seat's log: an arena seat answers over no socket, so the
//! session keeps no log for it.
//!
//! An answer the question's own check refuses (`Pending::answer_fault`) is
//! asked again once, with the reason, as the bridge does. One that fails
//! again, an error and an answer the engine refuses all go back to the
//! house, and each is counted.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context as _, anyhow};
use baylee_core::preset::SeatSpec;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_seat::llm::{ApiMind, Settings, Spec, credentials_at};
use baylee_seat::mind::{DeckCard, DeckList, GameContext, Mind as _, Refusal, RefusedBy, Request};
use baylee_view::{LogTail, PlayerView};

use crate::features::{PENDING_KINDS, question};
use crate::policy::{self, Picked};

/// How long the chair waits for one answer.
const BUDGET: Duration = Duration::from_secs(300);

/// A model in a chair, shared by every game of an arena.
pub struct LlmChair {
    mind: ApiMind,
    runtime: tokio::runtime::Runtime,
    /// The question kinds it answers (`features::PENDING_KINDS` names);
    /// `None` is every kind.
    kinds: Option<BTreeSet<String>>,
    spec: String,
    calls: AtomicU64,
    refused: AtomicU64,
    failed: AtomicU64,
    millis: AtomicU64,
}

impl LlmChair {
    /// `spec` as the seat names a model (`openai:<model>`), `base` the
    /// server's address (one on this machine may go without a key; one
    /// elsewhere takes the provider's key variable and TLS), and the
    /// question kinds it answers (empty: every kind).
    ///
    /// # Errors
    /// For a spec or address the seat refuses, an unknown kind, or a runtime
    /// that cannot start.
    pub fn new(
        spec: &str,
        base: &str,
        kinds: &[String],
        effort: Option<&str>,
        transcripts: Option<&std::path::Path>,
    ) -> anyhow::Result<Self> {
        let parsed = Spec::parse(spec)
            .context("--llm names openai:<model> or anthropic[:<model>]")?
            .map_err(|e| anyhow!(e))?;
        let credentials = credentials_at(
            parsed.provider,
            parsed.provider.default_key_env(),
            Some(base),
            &|k| std::env::var(k).ok(),
        )
        .map_err(|e| anyhow!(e))?;
        for kind in kinds {
            if !PENDING_KINDS.contains(&kind.as_str()) {
                anyhow::bail!("{kind} is no question kind; they are {PENDING_KINDS:?}");
            }
        }
        let mut settings = Settings::new(&parsed);
        if let Some(effort) = effort {
            settings.effort = Some(effort.to_owned());
        }
        settings.transcripts = transcripts.map(std::path::Path::to_path_buf);
        Ok(Self {
            mind: ApiMind::new(settings, credentials),
            runtime: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()?,
            kinds: (!kinds.is_empty()).then(|| kinds.iter().cloned().collect()),
            spec: spec.to_owned(),
            calls: AtomicU64::new(0),
            refused: AtomicU64::new(0),
            failed: AtomicU64::new(0),
            millis: AtomicU64::new(0),
        })
    }

    /// Whether the chair answers `pending`: one of its kinds, with more
    /// than one answer to choose from.
    #[must_use]
    pub fn takes(&self, pending: &Pending, view: &PlayerView) -> bool {
        let kind = PENDING_KINDS
            .get(usize::try_from(question(pending, view.hand.len()).0).unwrap_or(usize::MAX))
            .copied()
            .unwrap_or("");
        if self.kinds.as_ref().is_some_and(|k| !k.contains(kind)) {
            return false;
        }
        let hand: Vec<_> = view.hand.iter().map(|h| h.id).collect();
        policy::options(pending, &hand, &Picked::default()).map_or(true, |o| o.len() > 1)
    }

    /// The model's answer, or `None` when the house should answer instead.
    pub fn answer(
        &self,
        context: &Arc<GameContext>,
        question: u64,
        view: &PlayerView,
        pending: &Pending,
    ) -> Option<PlayerAction> {
        let mut retry = None;
        for _ in 0..2 {
            let request = Request {
                context: context.clone(),
                question,
                view: view.clone(),
                pending: pending.clone(),
                log: LogTail::default(),
                budget: BUDGET,
                retry: retry.take(),
                continuing: false,
            };
            self.calls.fetch_add(1, Ordering::Relaxed);
            let started = Instant::now();
            let got = self.runtime.block_on(self.mind.decide(request));
            self.millis.fetch_add(
                u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                Ordering::Relaxed,
            );
            match got {
                Ok(answer) => match pending.answer_fault(&answer.action) {
                    None => return Some(answer.action),
                    Some(fault) => {
                        self.refused.fetch_add(1, Ordering::Relaxed);
                        retry = Some(Refusal {
                            answer: answer.action,
                            reason: format!("{fault:?}"),
                            by: RefusedBy::Referee,
                        });
                    }
                },
                Err(e) => {
                    eprintln!("[llm] {}: {e:?}", self.spec);
                    break;
                }
            }
        }
        self.failed.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// An answer the engine refused after the check passed it: the house
    /// answers, and it is counted with the failures.
    pub fn engine_refused(&self) {
        self.failed.fetch_add(1, Ordering::Relaxed);
    }

    /// What a seat knows before the first card: its own deck.
    #[must_use]
    pub fn context(
        game_id: String,
        seat: &SeatSpec,
        at: u8,
        format: baylee_core::preset::FormatId,
    ) -> Arc<GameContext> {
        let main = crate::features3::deck_list(seat)
            .into_iter()
            .map(|(card, count)| DeckCard { card, count })
            .collect();
        Arc::new(GameContext {
            game_id,
            seat: baylee_core::ids::PlayerId::new(at),
            seats: 2,
            teams: vec![None, None],
            names: vec!["0".into(), "1".into()],
            format,
            deck: DeckList {
                name: String::new(),
                main,
                sideboard: Vec::new(),
                commanders: Vec::new(),
            },
            decision_secs: None,
        })
    }

    /// The chair's tallies for the report.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // counts stay far below 2^52
    pub fn report(&self) -> serde_json::Value {
        let calls = self.calls.load(Ordering::Relaxed);
        serde_json::json!({
            "model": self.spec,
            "kinds": self.kinds,
            "calls": calls,
            "refused_by_check": self.refused.load(Ordering::Relaxed),
            "to_the_house": self.failed.load(Ordering::Relaxed),
            "seconds_per_call": self.millis.load(Ordering::Relaxed) as f64 / 1000.0 / calls.max(1) as f64,
        })
    }
}
