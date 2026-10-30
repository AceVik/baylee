//! Transport-free neural seat: every authorized observation, every offered choice.
//! A transport break must terminate this adapter; a latest view cannot repair H.
mod audit;
mod brain;
mod draw;
pub use audit::LiveDecision4;
pub use brain::{Brain4, LiveAnswer4};
pub use draw::DrawDecision4;

use crate::features4::{SourceKind4, board_observation_hash, board_observation_sha256};
use crate::netplay4::NetPlayer4;
use anyhow::{Context as _, bail, ensure};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::Pending;
use baylee_protocol::v1::{self, Envelope};
use baylee_seat::{DeckList, TableMemory, memory::Heard};
use baylee_view::PlayerView;
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use std::borrow::Cow;

/// Match the current offline observation/candidate contract without changing
/// `TableMemory`'s authorized wire snapshot. Archived Encoder 6 models strip
/// `casting` and `targeting`, while Encoder 7 models retain them natively.
fn policy_view(view: &PlayerView, retain_context: bool) -> Cow<'_, PlayerView> {
    if retain_context || (view.casting.is_none() && view.targeting.is_none()) {
        return Cow::Borrowed(view);
    }
    let mut compatible = view.clone();
    compatible.casting = None;
    compatible.targeting = None;
    Cow::Owned(compatible)
}

/// Honest local metadata; the existing wire `SeatMind` enum has no neural kind.
#[derive(Clone, Debug, Default, Serialize)]
pub struct LiveReport4 {
    /// Always a local neural player, never mislabeled as an LLM or script.
    pub kind: &'static str,
    /// Complete initialization count (one for a live game).
    pub initializations: u64,
    /// Distinct authorized observations consumed at K=0.
    pub observations: u64,
    /// Identical authorized observations ignored.
    pub duplicate_observations: u64,
    /// Distinct questions answered after the curtain.
    pub answers: u64,
    /// Answers actually selected by the neural policy.
    pub neural_answers: u64,
    /// Forced answers returned by the runtime.
    pub forced_answers: u64,
    /// Explicit house/runtime fallback answers.
    pub fallback_answers: u64,
    /// Search-selected answers (distinct from neural and house fallbacks).
    pub search_answers: u64,
    /// Draw offers answered by the explicit live rule over the neural critic.
    pub draw_offer_answers: u64,
    /// Last rule evaluation; no player identity or hidden information included.
    pub last_draw_decision: Option<DrawDecision4>,
    /// Repeated/stale questions ignored without another policy call.
    pub duplicate_questions: u64,
    /// Server clock-probe echoes heard (never echoed back).
    pub clock_probes: u64,
    /// Fail-closed reason, with no transport credentials.
    pub failure: Option<String>,
}

struct Question {
    seq: u64,
    raw: Vec<u8>,
    pending: Box<Pending>,
}

/// One live game and one seat. No socket, wake filter, wall clock or secret.
pub struct LiveSeat4<B = NetPlayer4> {
    brain: B,
    deck: DeckList,
    seat: PlayerId,
    memory: TableMemory,
    game_id: Option<String>,
    teams: Vec<Option<u8>>,
    last_observation: Option<u64>,
    history: Sha256,
    last_decision: Option<LiveDecision4>,
    last_answered: Option<(u64, Vec<u8>)>,
    waiting: Option<Question>,
    ready: bool,
    curtain: bool,
    done: bool,
    report: LiveReport4,
}

impl<B: Brain4> LiveSeat4<B> {
    /// Binds the brain to a known chair and that chair's own deck.
    #[must_use]
    pub fn new(brain: B, deck: DeckList, seat: PlayerId) -> Self {
        Self {
            brain,
            deck,
            seat,
            memory: TableMemory::default(),
            game_id: None,
            teams: Vec::new(),
            last_observation: None,
            history: Sha256::new(),
            last_decision: None,
            last_answered: None,
            waiting: None,
            ready: false,
            curtain: false,
            done: false,
            report: LiveReport4 {
                kind: "local_neural_v4",
                ..LiveReport4::default()
            },
        }
    }

    /// Consumes one protobuf frame and returns authorized outgoing envelopes.
    /// Any observation/protocol/answer error permanently closes this adapter.
    ///
    /// # Errors
    /// Returns an error for incompatible or malformed frames, failed observation
    /// inference, invalid offered answers, or an explicit table refusal.
    pub fn hear(&mut self, bytes: &[u8]) -> anyhow::Result<Vec<Envelope>> {
        if self.done {
            return Ok(Vec::new());
        }
        let result = self.hear_inner(bytes);
        if let Err(error) = &result {
            self.done = true;
            self.report.failure = Some(error.to_string());
        }
        result
    }

    /// True after game over, a transport gap or any fail-closed error.
    #[must_use]
    pub const fn finished(&self) -> bool {
        self.done
    }

    /// Counters suitable for local diagnostics; no socket credentials included.
    #[must_use]
    pub const fn report(&self) -> &LiveReport4 {
        &self.report
    }

    /// Latest generated action audit; present only until the next incoming frame.
    /// Generation does not establish socket delivery or engine acceptance.
    #[must_use]
    pub const fn decision_audit(&self) -> Option<&LiveDecision4> {
        self.last_decision.as_ref()
    }

    /// A live socket gap cannot silently reuse incomplete recurrent memory.
    pub fn disconnected(&mut self) {
        self.done = true;
        self.report.failure =
            Some("seat transport interrupted; recurrent history is incomplete".into());
    }

    fn hear_inner(&mut self, bytes: &[u8]) -> anyhow::Result<Vec<Envelope>> {
        self.last_decision = None;
        self.brain.validate_live_contract()?;
        let heard = self.memory.hear(bytes);
        let outgoing = Vec::new();
        match heard {
            Heard::Statics => {
                let statics = self.memory.statics().context("missing game setup")?;
                ensure!(
                    statics.your_seat == self.seat,
                    "game setup names another seat"
                );
                let teams: Vec<_> = statics.seats.iter().map(|seat| seat.team).collect();
                if let Some(game) = &self.game_id {
                    ensure!(
                        *game == statics.game_id && self.teams == teams,
                        "live game identity changed"
                    );
                } else {
                    ensure!(
                        !teams.is_empty()
                            && teams.len() <= 8
                            && usize::from(self.seat.get()) < teams.len(),
                        "invalid live roster"
                    );
                    self.brain.initialize(self.seat, &self.deck, &teams)?;
                    let initial = crate::features4::SeatKnowledge4::from_deck_list(
                        self.seat,
                        teams.len(),
                        &self.deck,
                    );
                    audit::extend(
                        &mut self.history,
                        b"live_authorized_history_v1/init",
                        &serde_json::to_vec(&(
                            self.seat,
                            &initial.own_deck_cards,
                            &initial.inventory,
                            &teams,
                        ))?,
                    );
                    self.report.initializations += 1;
                    self.game_id = Some(statics.game_id.clone());
                    self.teams = teams;
                }
                self.observe_latest()?;
            }
            Heard::View => {
                self.observe_latest()?;
            }
            Heard::Question { seq, pending, raw } => {
                if matches!(*pending, Pending::GameOver(_)) {
                    self.done = true;
                    return Ok(outgoing);
                }
                if pending.asked() != Some(self.seat) {
                    return Ok(outgoing);
                }
                if let Some((last, old_raw)) = &self.last_answered {
                    // Other seats' simultaneous opening answers advance the
                    // global seq while our already-sent opening answer waits.
                    // A new mulligan changes its count; bottoming happens once.
                    let opening_repeat = raw == *old_raw
                        && matches!(
                            *pending,
                            Pending::Mulligan { .. } | Pending::MulliganBottom { .. }
                        );
                    if seq < *last || (seq == *last && raw == *old_raw) || opening_repeat {
                        self.report.duplicate_questions += 1;
                        return Ok(outgoing);
                    }
                    ensure!(seq != *last, "same question sequence changed its content");
                }
                self.waiting = Some(Question { seq, raw, pending });
            }
            Heard::Curtain => {
                self.curtain = true;
            }
            Heard::ClockProbe { .. } => {
                self.report.clock_probes += 1;
            }
            Heard::Refused(message) => {
                let class = match message.as_str() {
                    "not a human seat" => "not a human seat",
                    "illegal action for your seat" => "illegal action for your seat",
                    _ => "unclassified server refusal (text omitted)",
                };
                bail!("table refused neural seat action: {class}; refusing silent fallback")
            }
            Heard::Incompatible(_) => bail!("incompatible live protocol or view version"),
            Heard::Unreadable(_) => bail!("unreadable live seat frame"),
            Heard::Other => {}
        }
        self.settle()
    }

    fn settle(&mut self) -> anyhow::Result<Vec<Envelope>> {
        let mut outgoing = Vec::new();
        if self.game_id.is_some() && self.memory.view().is_some() && !self.ready {
            self.ready = true;
            outgoing.push(Envelope {
                msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
            });
        }
        if self.curtain
            && self.ready
            && let Some(question) = self.waiting.take()
        {
            let view = self
                .memory
                .view()
                .context("question arrived without an authorized view")?;
            if view.seq < question.seq {
                self.waiting = Some(question);
                return Ok(outgoing);
            }
            if view.seq > question.seq {
                self.report.duplicate_questions += 1;
                return Ok(outgoing);
            }
            let view = policy_view(view, self.brain.retains_live_context());
            let answer = self
                .brain
                .answer(view.as_ref(), &question.pending, &self.teams)?;
            ensure!(
                question.pending.answer_fault(&answer.action).is_none(),
                "brain returned an invalid offered answer"
            );
            ensure!(
                answer.provenance.source != SourceKind4::External,
                "external actions cannot be attributed to a live neural brain"
            );
            audit::validate(answer.provenance)?;
            let action_json = serde_json::to_vec(&answer.action)?;
            let pending_json = serde_json::to_vec(&question.pending)?;
            let decision = LiveDecision4 {
                schema: "live_decision_audit_v1",
                decision: self.report.answers + 1,
                seq: question.seq,
                observation: self.report.observations,
                view_sha256: audit::digest(&serde_json::to_vec(view.as_ref())?),
                observation_sha256: audit::hex(&board_observation_sha256(view.as_ref())),
                pending_sha256: audit::digest(&pending_json),
                action_sha256: audit::digest(&action_json),
                history_sha256: audit::hex(&self.history.clone().finalize()),
                provenance: answer.provenance,
            };
            audit::extend(&mut self.history, b"question", &pending_json);
            audit::extend(&mut self.history, b"action", &action_json);
            self.last_decision = Some(decision);
            self.report.answers += 1;
            if let Some(draw) = answer.draw {
                self.report.draw_offer_answers += 1;
                self.report.last_draw_decision = Some(draw);
            }
            match answer.provenance.source {
                SourceKind4::Net => self.report.neural_answers += 1,
                SourceKind4::Forced => self.report.forced_answers += 1,
                SourceKind4::House => self.report.fallback_answers += 1,
                SourceKind4::Search => self.report.search_answers += 1,
                SourceKind4::External => {
                    anyhow::bail!("external actions cannot be attributed to a live neural brain")
                }
            }
            outgoing.push(Envelope {
                msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                    game_id: self.game_id.clone().unwrap_or_default(),
                    seat_token: String::new(),
                    action_json,
                })),
            });
            self.last_answered = Some((question.seq, question.raw));
        }
        Ok(outgoing)
    }

    fn observe_latest(&mut self) -> anyhow::Result<()> {
        if self.game_id.is_none() {
            return Ok(());
        }
        let Some(view) = self.memory.view() else {
            return Ok(());
        };
        ensure!(
            view.seat == self.seat && view.seats.len() == self.teams.len(),
            "view is not authorized for this seat/roster"
        );
        let view = policy_view(view, self.brain.retains_live_context());
        let digest = board_observation_hash(view.as_ref());
        if self.last_observation == Some(digest) {
            self.report.duplicate_observations += 1;
            return Ok(());
        }
        self.brain
            .observe(view.as_ref())
            .context("neural observation failed; recurrent trajectory stopped")?;
        audit::extend(
            &mut self.history,
            b"observation",
            &board_observation_sha256(view.as_ref()),
        );
        self.last_observation = Some(digest);
        self.report.observations += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
