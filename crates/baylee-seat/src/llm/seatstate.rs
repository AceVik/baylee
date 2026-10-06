//! What a language model's seat keeps between decisions, whoever holds
//! the conversation: the API mind ([`super::ApiMind`]) keeps the messages
//! itself, a CLI ([`crate::cli::CliMind`]) keeps them in its own process.
//! Both tell the same decision the same way and act on an answer the same
//! way: a plan's taps and a hint's targets go out without a call, a cast
//! the table took back is said, a refusal and a late answer are noted for
//! the next message, and the model's own `say` lines are carried into the
//! next turn as its notes.

use super::SAYS;
use crate::mind::{Answer, GameContext, Request};
use crate::narrator::{self, Act, Hint, Narrator};
use crate::transcript::Transcript;
use baylee_client_core::manaplan;
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction, TargetPrompt};
use baylee_view::{LogEvent, LogObject};
use serde_json::json;
use std::collections::VecDeque;
use std::path::Path;
use std::time::Duration;

/// Taps still to send, and what they pay for.
#[derive(Clone, Debug)]
pub(crate) struct Plan {
    steps: VecDeque<manaplan::Step>,
    then: PlayerAction,
    /// The option, in words.
    label: String,
    /// The step it was chosen in: a plan never runs into another.
    at: (u32, baylee_view::Phase, baylee_view::Step),
    /// The colour the last tap makes, for the question it may ask.
    color: Option<ManaColor>,
}

/// What one seat holds between decisions, apart from the conversation.
pub(crate) struct Seat {
    pub(crate) narrator: Narrator,
    /// The turn of the last answer the model gave; 0 before the first.
    pub(crate) turn: u32,
    /// Whether the last answer sent was the model's own (not a plan's).
    pub(crate) last_by_model: bool,
    plan: Option<Plan>,
    hint: Option<Hint>,
    /// Lines for the next message, under its header.
    pub(crate) notes: Vec<String>,
    /// The model's own earlier `say` lines, newest last.
    says: VecDeque<String>,
    /// A question whose answer came after its time.
    pub(crate) late: Option<u64>,
    /// How many calls this seat has started, to tell a stale one.
    pub(crate) asked: u64,
    /// The card this seat last answered a cast of, until the log or the
    /// next priority shows whether the cast happened ([`Seat::undone`]).
    pub(crate) casting: Option<ObjectId>,
    pub(crate) transcript: Transcript,
    pub(crate) stops: Option<narrator::Stops>,
    pub(crate) hold: Option<String>,
}

impl Seat {
    /// A seat at the start of `context`'s game, writing its transcript as
    /// `<game>-seat<n>-mind.jsonl` in `transcripts`, if that is given.
    pub(crate) fn new(
        context: &GameContext,
        transcripts: Option<&Path>,
        style: narrator::Style,
    ) -> Self {
        let transcript = transcripts.map_or_else(Transcript::none, |dir| {
            let path = dir.join(format!(
                "{}-seat{}-mind.jsonl",
                context.game_id,
                context.seat.get()
            ));
            Transcript::file(&path).unwrap_or_else(|_| Transcript::none())
        });
        Self {
            narrator: Narrator::styled(context, style),
            turn: 0,
            last_by_model: false,
            plan: None,
            hint: None,
            notes: Vec::new(),
            says: VecDeque::new(),
            late: None,
            asked: 0,
            casting: None,
            transcript,
            stops: None,
            hold: None,
        }
    }

    /// What is done before the model is asked: the log heard, a cast taken
    /// back, a late answer or a refusal noted, and the plan or the hint
    /// followed. `Some` is the answer when no call is needed.
    ///
    /// A refusal of the model's own answer is handed to `carry` first,
    /// which takes it where the conversation has a place for it (a tool's
    /// result) and says so; else it goes into the notes.
    pub(crate) fn begin(
        &mut self,
        request: &Request,
        carry: impl FnOnce(&str) -> bool,
    ) -> Option<Answer> {
        self.narrator.hear(&request.log);
        if let Some(undone) = self.undone(request) {
            self.notes.push(undone);
        }
        if let Some(question) = self.late.take() {
            self.notes.push(format!(
                "Your answer to q{question} came after its time ran out; the house answered it."
            ));
        }
        if let Some(refusal) = &request.retry {
            self.refused(&refusal.reason, &refusal.answer, carry);
        }
        let (action, label) = self.follow(request)?;
        self.last_by_model = false;
        let note = json!({"plan": label}).to_string();
        let stops = self.stops.clone();
        let hold = self.hold.clone();
        Some(Answer {
            action,
            model_time: Duration::ZERO,
            note: Some(note),
            thinking: None,
            stops: stops.map(Box::new),
            hold,
        })
    }

    /// The lines a message carries above the decision: at the start of a
    /// conversation the model's notes from earlier turns, and always what
    /// it has not yet been told. The notes are kept until the model
    /// answers: a message it never read is told again.
    pub(crate) fn told(&self, fresh: bool) -> Vec<String> {
        let mut told = Vec::new();
        if fresh && !self.says.is_empty() {
            let said: Vec<String> = self.says.iter().map(|s| format!("  - {s}")).collect();
            told.push(format!(
                "Your notes from earlier turns:\n{}",
                said.join("\n")
            ));
        }
        told.extend(self.notes.iter().cloned());
        told
    }

    pub(crate) fn stops_summary(&self) -> Option<String> {
        let mut s = String::new();
        if let Some(stops) = &self.stops {
            s.push_str("stops: mine [");
            s.push_str(&stops.mine.join(", "));
            s.push_str("], theirs [");
            s.push_str(&stops.theirs.join(", "));
            s.push(']');
        }
        if let Some(hold) = &self.hold {
            if !s.is_empty() {
                s.push_str(" · ");
            }
            s.push_str("hold: ");
            s.push_str(hold);
        }
        if s.is_empty() { None } else { Some(s) }
    }

    /// Keeps what the model answered, `resolved` against its question and
    /// with its `say`, told by `narrator`, and turns it into the seat's
    /// answer: the action now, or a plan's first tap.
    pub(crate) fn keep(
        &mut self,
        request: &Request,
        narrator: Narrator,
        resolved: narrator::Resolved,
        say: Option<&str>,
    ) -> PlayerAction {
        self.narrator = narrator;
        self.turn = request.view.turn;
        self.notes.clear();
        self.last_by_model = true;
        if let Some(say) = say {
            self.says.push_back(say.chars().take(300).collect());
            while self.says.len() > SAYS {
                self.says.pop_front();
            }
        }
        self.hint.clone_from(&resolved.hint);
        if let Some(stops) = resolved.stops {
            self.stops = Some(stops);
        }
        if resolved.hold.is_some() {
            self.hold = resolved.hold;
        } else {
            // "hold" expires when the seat gets a chance to act on it (a new turn),
            // or when they send a new answer without it. If the user explicitly sends an answer,
            // the hold should be dropped. But wait, `hold: "until_my_turn"` is a one-off instruction.
            self.hold = None;
        }
        match resolved.act {
            Act::Now(action) => {
                self.plan = None;
                action
            }
            Act::Taps { steps, then } => {
                let mut steps: VecDeque<manaplan::Step> = steps.into();
                let first = steps.pop_front();
                let view = &request.view;
                self.plan = Some(Plan {
                    steps,
                    then: then.clone(),
                    label: resolved.label,
                    at: (view.turn, view.phase, view.step),
                    color: first.and_then(|s| s.color),
                });
                if let Some(step) = first {
                    narrator::tap(&step)
                } else {
                    self.plan = None;
                    then
                }
            }
        }
    }

    /// What to tell the model when the cast it last answered did not
    /// happen. The table takes back a cast whose whole cost cannot be paid
    /// and gives priority back (CR 601.2h, 732.1, 732.2) without a word, so
    /// the model would see the same question again and may answer it the
    /// same way, again.
    ///
    /// A cast that happened is in the log ([`LogEvent::Cast`]), which the
    /// mind is handed whole, a line at a time: from the cast's answer on,
    /// every request's lines are read for it. One that did not reach the
    /// log by the next priority, with its card still in the hand or the
    /// command zone, was taken back. The card's place alone would not say
    /// so: an object keeps its handle across zones, so a spell that
    /// resolved and came back to the hand stands where it was cast from.
    /// A refused answer says why itself, and gets no second reason.
    fn undone(&mut self, request: &Request) -> Option<String> {
        let card = self.casting?;
        let cast = request.log.entries.iter().any(|entry| {
            matches!(
                &entry.event,
                LogEvent::Cast { spell: LogObject::Known { id, .. }, .. } if *id == card
            )
        });
        if cast {
            self.casting = None;
            return None;
        }
        if !matches!(request.pending, Pending::Priority { .. }) {
            return None;
        }
        self.casting = None;
        if request.retry.is_some() {
            return None;
        }
        let view = &request.view;
        let name = view
            .hand
            .iter()
            .find(|c| c.id == card)
            .map(|c| c.name.clone())
            .or_else(|| {
                view.command
                    .iter()
                    .flatten()
                    .find(|o| o.id == card)
                    .map(|o| o.name.clone())
            })?;
        Some(format!(
            "Your cast of {name} {} did not happen: the card is where it was, and you have \
             priority again. The table takes back a cast whose whole cost cannot be paid, \
             with any kicker or additional cost you said yes to (CR 601.2h, 732.1); count \
             the cost before you cast it again.",
            narrator::tag(card)
        ))
    }

    /// The table or the referee refused the seat's last answer: the plan
    /// and the hint are dropped, and the model is told, through `carry`
    /// where it takes the sentence, else in the notes.
    fn refused(&mut self, reason: &str, action: &PlayerAction, carry: impl FnOnce(&str) -> bool) {
        self.plan = None;
        self.hint = None;
        if !self.last_by_model {
            self.notes.push(format!(
                "Your plan stopped: the table refused {action:?} ({reason})."
            ));
            return;
        }
        let said = format!("Your answer was refused: {reason}. Answer the same question again.");
        if !carry(&said) {
            self.notes.push(said);
        }
    }

    /// The answer the plan or the hint gives, when either fits.
    fn follow(&mut self, request: &Request) -> Option<(PlayerAction, String)> {
        if let Some(action) = self.follow_plan(request) {
            let label = self
                .plan
                .as_ref()
                .map_or_else(|| "the plan's last step".to_string(), |p| p.label.clone());
            return Some((action, label));
        }
        if self.plan.is_none() && matches!(request.pending, Pending::Priority { .. }) {
            self.hint = None;
        }
        self.follow_hint(request)
            .map(|action| (action, "the targets named ahead".to_string()))
    }

    fn follow_plan(&mut self, request: &Request) -> Option<PlayerAction> {
        let plan = self.plan.as_mut()?;
        let view = &request.view;
        let same = (view.turn, view.phase, view.step) == plan.at;
        match &request.pending {
            Pending::ChooseColor { options, .. } if same => {
                if let Some(color) = plan.color.take().filter(|c| options.contains(c)) {
                    return Some(PlayerAction::ChooseColor(color));
                }
            }
            Pending::Priority { legal, .. } if same => {
                if let Some(step) = plan.steps.front().copied() {
                    let action = narrator::tap(&step);
                    if offered(legal, &action) {
                        plan.steps.pop_front();
                        plan.color = step.color;
                        return Some(action);
                    }
                } else if offered(legal, &plan.then) {
                    let then = plan.then.clone();
                    self.plan = None;
                    return Some(then);
                }
            }
            _ => {}
        }
        let plan = self.plan.take()?;
        self.notes.push(format!(
            "Your plan ({}) stopped before it finished: the table asked something else. Any \
             mana it made is in your pool.",
            plan.label
        ));
        None
    }

    fn follow_hint(&mut self, request: &Request) -> Option<PlayerAction> {
        let hint = self.hint.take()?;
        let Pending::ChooseTargets {
            options,
            player_options,
            min,
            max,
            reason: TargetPrompt::Targets,
            ..
        } = &request.pending
        else {
            // A cast mode, an X or a colour may come before the targets.
            if !matches!(request.pending, Pending::Priority { .. }) {
                self.hint = Some(hint);
            }
            return None;
        };
        let targeting = request.view.targeting.as_ref()?;
        let card = targeting
            .source
            .rules
            .map(|r| r.card)
            .or_else(|| targeting.source.card.map(|c| c.index));
        let count = hint.objects.len() + hint.players.len();
        let fits = card == Some(hint.card)
            && !targeting.second
            && (usize::try_from(*min).unwrap_or(usize::MAX)
                ..=usize::try_from(*max).unwrap_or(usize::MAX))
                .contains(&count)
            && hint.objects.iter().all(|o| options.contains(o))
            && hint.players.iter().all(|p| player_options.contains(p));
        if !fits {
            self.notes
                .push("The targets you named ahead are not legal now: choose them here.".into());
            return None;
        }
        Some(PlayerAction::ChooseTargets {
            objects: hint.objects,
            players: hint.players,
        })
    }
}

/// Whether `legal` offers `action` now.
fn offered(legal: &LegalActions, action: &PlayerAction) -> bool {
    match action {
        PlayerAction::ActivateManaAbility { source } => legal.mana_abilities.contains(source),
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => legal.abilities.contains(&(*source, *ability_index)),
        PlayerAction::CastSpell { card } => legal.castable.contains(card),
        PlayerAction::PassPriority => legal.can_pass,
        _ => false,
    }
}
