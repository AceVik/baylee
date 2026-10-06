//! What a language model's seat keeps between decisions, whoever holds
//! the conversation: the API mind ([`super::ApiMind`]) keeps the messages
//! itself, a CLI ([`crate::cli::CliMind`]) keeps them in its own process.
//! Both tell the same decision the same way and act on an answer the same
//! way: a plan's steps, an option's taps and a hint's targets go out
//! without a call, a cast the table took back is said, a refusal and a
//! late answer are noted for the next message, and the model's own `say`
//! lines are carried into the next turn as its notes.
//!
//! # A plan's steps
//!
//! The steps the model wrote with an answer ([`narrator::Step`]) answer
//! the questions after it, one step a question, each read against that
//! question's own offer as an answer of the model's is
//! ([`narrator::Menu::plan_decision`]). The seat passes priority for a
//! plan only toward what its next step answers: while nothing but its own
//! spells and abilities is on the stack (they resolve, and ask what the
//! step answers), and on its own turn before combat when the next step is
//! the attack. Anything else stops the plan, and the model is asked, with
//! the reason in the next message (`docs/llm-protocol.md` §"Plans"): a
//! question of another kind, an id or a name the offer does not hold,
//! another side's spell or ability on the stack, a payment owed, the end
//! of its turn, a refusal. Nothing of a plan is ever guessed.

use super::SAYS;
use crate::mind::{Answer, GameContext, Request};
use crate::narrator::{self, Act, Hint, Narrator, Step, Stop, Table};
use crate::transcript::Transcript;
use baylee_client_core::manaplan;
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction, TargetPrompt};
use baylee_view::{LogEvent, LogObject, PlayerView};
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

/// The steps of a plan the model wrote, still to run, and what it did.
#[derive(Clone, Debug)]
struct Queue {
    steps: VecDeque<Step>,
    /// The question it was written at.
    written: u64,
    /// The turn it runs in.
    turn: u32,
    /// The number of the next step, from 1.
    next: usize,
    /// What it did, in order: steps, and windows passed.
    done: Vec<String>,
    /// Windows passed since the last step.
    windows: u32,
    /// The step the seat sent last, until the next request shows the
    /// table took it.
    sent: Option<Step>,
}

impl Queue {
    /// Ends the count of windows passed, into what it did.
    fn settle(&mut self) {
        match std::mem::take(&mut self.windows) {
            0 => {}
            1 => self.done.push("passed 1 window".into()),
            n => self.done.push(format!("passed {n} windows")),
        }
    }

    /// The report of a plan that ran to its end.
    fn ran(mut self) -> String {
        self.settle();
        format!("Plan q{} ran: {}.", self.written, self.done.join(" · "))
    }

    /// The report of a plan that stopped at its next step.
    fn stopped(mut self, why: &Stop) -> String {
        self.settle();
        let step = self
            .steps
            .front()
            .map_or_else(String::new, |step| format!(" ({step})"));
        let head = if self.done.is_empty() {
            format!("Plan q{} stopped", self.written)
        } else {
            format!(
                "Plan q{} ran: {}. Stopped",
                self.written,
                self.done.join(" · ")
            )
        };
        format!(
            "{head} at step {}{step}: {}; the rest of the plan is dropped, and this question is \
             yours.",
            self.next,
            why.sentence()
        )
    }
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
    /// The steps of the model's plan still to run.
    queue: Option<Queue>,
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
            queue: None,
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
        // A hold lasts until the seat's turn, or the model's next answer
        // without it.
        self.hold =
            (resolved.until == Some(narrator::Until::MyTurn)).then(|| "until_my_turn".to_string());
        let view = &request.view;
        self.queue = (!resolved.plan.is_empty()).then(|| Queue {
            steps: resolved.plan.into(),
            written: request.question,
            turn: view.turn,
            next: 1,
            done: Vec::new(),
            windows: 0,
            sent: None,
        });
        self.start(resolved.act, resolved.label, view)
    }

    /// The first action of `act`: the action itself, or the first of its
    /// taps, the rest kept to answer the priorities that follow.
    fn start(&mut self, act: Act, label: String, view: &PlayerView) -> PlayerAction {
        match act {
            Act::Now(action) => {
                self.plan = None;
                action
            }
            Act::Taps { steps, then } => {
                let mut steps: VecDeque<manaplan::Step> = steps.into();
                let first = steps.pop_front();
                self.plan = Some(Plan {
                    steps,
                    then: then.clone(),
                    label,
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

    /// Ends the plan at its next step for `why`, and says so in the next
    /// message.
    fn stop(&mut self, why: &Stop) {
        if let Some(queue) = self.queue.take() {
            self.notes.push(queue.stopped(why));
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
            if let Some(queue) = &mut self.queue {
                // The refused answer was the step sent last: the plan
                // stops there.
                if let Some(step) = queue.sent.take() {
                    queue.done.pop();
                    queue.steps.push_front(step);
                    queue.next -= 1;
                }
                self.stop(&Stop::Refused(format!("{action:?}: {reason}")));
            } else {
                self.notes.push(format!(
                    "Your plan stopped: the table refused {action:?} ({reason})."
                ));
            }
            return;
        }
        // The answer the plan came with was refused: the model answers
        // again, with a plan again if it likes.
        self.queue = None;
        let said = format!("Your answer was refused: {reason}. Answer the same question again.");
        if !carry(&said) {
            self.notes.push(said);
        }
    }

    /// The answer the taps in flight, the hint or the plan's next step
    /// gives, when one fits.
    fn follow(&mut self, request: &Request) -> Option<(PlayerAction, String)> {
        if let Some(action) = self.follow_plan(request) {
            let label = self
                .plan
                .as_ref()
                .map_or_else(|| "the taps' last step".to_string(), |p| p.label.clone());
            return Some((action, label));
        }
        if self.plan.is_none() && matches!(request.pending, Pending::Priority { .. }) {
            self.hint = None;
        }
        if let Some(action) = self.follow_hint(request) {
            return Some((action, "the targets named ahead".to_string()));
        }
        self.follow_queue(request)
    }

    /// The answer the plan's next step gives this question, or why the
    /// plan stops here (`docs/llm-protocol.md` §"Plans").
    fn follow_queue(&mut self, request: &Request) -> Option<(PlayerAction, String)> {
        let queue = self.queue.as_mut()?;
        queue.sent = None;
        let view = &request.view;
        let table = Table::new(view, &request.context);
        if view.turn != queue.turn {
            self.stop(&Stop::TurnOver);
            return None;
        }
        if let Some(theirs) = view.stack.iter().find(|o| !table.ally(o.controller)) {
            let what = format!(
                "{}'s {}",
                Table::player_id(theirs.controller),
                table.named(theirs.id)
            );
            self.stop(&Stop::OpposingStack(what));
            return None;
        }
        if view.owed.is_some() {
            self.stop(&Stop::Owed);
            return None;
        }
        loop {
            let queue = self.queue.as_mut()?;
            let step = queue.steps.front()?.clone();
            if let Pending::Priority { legal, .. } = &request.pending
                && !step.at_priority()
            {
                let mine = view.active == view.seat;
                let before_combat = matches!(
                    view.phase,
                    baylee_view::Phase::Beginning | baylee_view::Phase::FirstMain
                ) || view.step == baylee_view::Step::CombatBegin;
                let past_attack = mine
                    && view.phase == baylee_view::Phase::Combat
                    && view.step != baylee_view::Step::CombatBegin;
                let toward = !view.stack.is_empty()
                    || matches!(step, Step::Attack(_)) && mine && before_combat;
                if toward && legal.can_pass && view.step != baylee_view::Step::Cleanup {
                    queue.windows += 1;
                    return Some((PlayerAction::PassPriority, plan_label(queue)));
                }
                if past_attack && step == Step::Attack(Vec::new()) {
                    // Nothing attacked, as the step said: the table
                    // declared it for the seat (nothing could attack).
                    queue.settle();
                    queue.done.push(step.to_string());
                    queue.steps.pop_front();
                    queue.next += 1;
                    if queue.steps.is_empty() {
                        let ran = self.queue.take().expect("a plan").ran();
                        self.notes.push(ran);
                        return None;
                    }
                    continue;
                }
                if past_attack && matches!(step, Step::Attack(_)) {
                    self.stop(&Stop::AttackPassed);
                    return None;
                }
            }
            let menu = self.narrator.menu(request);
            let resolved = menu
                .plan_decision(&step, request)
                .and_then(|decision| menu.resolve(&decision).map_err(Stop::NotOffered));
            let resolved = match resolved {
                Ok(resolved) => resolved,
                Err(why) => {
                    self.stop(&why);
                    return None;
                }
            };
            let queue = self.queue.as_mut()?;
            queue.settle();
            queue.done.push(step.to_string());
            queue.steps.pop_front();
            queue.next += 1;
            queue.sent = Some(step.clone());
            let label = format!("step {} of q{}: {step}", queue.next - 1, queue.written);
            if queue.steps.is_empty() {
                let ran = self.queue.take().expect("a plan").ran();
                self.notes.push(ran);
            }
            self.hint.clone_from(&resolved.hint);
            let action = self.start(resolved.act, label.clone(), view);
            return Some((action, label));
        }
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
        if self.queue.is_some() {
            self.stop(&Stop::TapsStopped);
        } else {
            self.notes.push(format!(
                "Your plan ({}) stopped before it finished: the table asked something else. Any \
                 mana it made is in your pool.",
                plan.label
            ));
        }
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
            if self.queue.is_some() {
                self.stop(&Stop::TargetsNotLegal);
            } else {
                self.notes.push(
                    "The targets you named ahead are not legal now: choose them here.".into(),
                );
            }
            return None;
        }
        Some(PlayerAction::ChooseTargets {
            objects: hint.objects,
            players: hint.players,
        })
    }
}

/// The label of a window a plan passes.
fn plan_label(queue: &Queue) -> String {
    format!(
        "a window passed toward step {} of q{}",
        queue.next, queue.written
    )
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
