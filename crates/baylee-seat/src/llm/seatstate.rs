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

use super::prompt::{LOOKUPS_PER_GAME, LOOKUPS_PER_QUESTION};
use super::{RETRY_FLOOR, SAYS, Settings, lock};
use crate::lookup::{self, Library};
use crate::mind::{Answer, GameContext, Request};
use crate::narrator::{self, Act, Decision, Hint, Menu, Narrator, Step, Stop, Table};
use crate::reports::{self, Report};
use crate::transcript::Transcript;
use crate::wake::{Held, Orders, Why};
use baylee_client_core::manaplan;
use baylee_core::ids::ObjectId;
use baylee_core::mana::ManaColor;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction, TargetPrompt};
use baylee_view::{LogEvent, LogObject, PlayerView};
use serde_json::json;
use std::collections::{BTreeSet, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex};
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
    /// The lookups this seat made in its game.
    lookups: u32,
    /// The cards this seat's model was told the build's own text of by a
    /// lookup, by name folded ([`lookup::fold`]): a report's quote of one
    /// is checked against that text too.
    looked_up: BTreeSet<String>,
    /// The reports this seat filed in its game, until the bridge takes
    /// them.
    pub(crate) reports: reports::Book,
}

/// A report or a lookup, as the seat that made it sees it: the question
/// and how it was told, the model, where reports are written.
pub(crate) struct Making<'a> {
    request: &'a Request,
    settings: &'a Settings,
    /// The question's options, as the model was shown them.
    menu: &'a Menu,
    /// The narrator that told the question: the card texts the model was
    /// told, this question's new cards included.
    narrator: &'a Narrator,
    /// The lookups made for this question so far.
    looked: u32,
}

impl<'a> Making<'a> {
    /// What the seat answering `request` with `settings` makes, the
    /// question told by `narrator` with `menu`, after `looked` lookups for
    /// the question.
    pub(crate) const fn new(
        request: &'a Request,
        settings: &'a Settings,
        menu: &'a Menu,
        narrator: &'a Narrator,
        looked: u32,
    ) -> Self {
        Self {
            request,
            settings,
            menu,
            narrator,
            looked,
        }
    }
}

/// What holds a seat between decisions with its conversation: the API's
/// and a CLI's.
pub(crate) trait Holds: Send {
    /// The seat.
    fn seat(&mut self) -> &mut Seat;
}

/// What became of the lookup an answer asked for.
pub(crate) enum Looked {
    /// It asked for none.
    Nothing,
    /// It was made: the message that tells the model the card and asks
    /// the question again.
    Made(String),
    /// It may not be made now: why, said where the answer does not answer.
    Refused(String),
}

/// What a language model's seat does with an answer's `decision` before
/// it reads it against its question, whoever holds the conversation: the
/// report it carries filed, and the lookup it asks for made where it may
/// be ([`Seat::may_look_up`], with `left` of the question's time) in
/// `library`, off the executor. The seat is never held while the card is
/// looked up.
///
/// Only an answer of `ask` and `lookup` alone asks for a card: one that
/// also answers its question ([`Decision::answers`]) is taken as it is,
/// and its lookup is passed over (a line of the mind's transcript), never
/// made at the cost of the answer.
pub(crate) async fn report_and_look_up<T: Holds>(
    holder: &Mutex<T>,
    library: &Arc<Library>,
    making: &Making<'_>,
    decision: Option<&Decision>,
    left: Duration,
) -> Looked {
    let Some(decision) = decision else {
        return Looked::Nothing;
    };
    let (name, timeout) = {
        let mut held = lock(holder);
        let seat = held.seat();
        if let Some(report) = &decision.report {
            seat.file_report(report, decision, making);
        }
        let Some(name) = decision.lookup.clone() else {
            return Looked::Nothing;
        };
        if decision.answers() {
            seat.passed_over(making, &name);
            return Looked::Nothing;
        }
        match seat.may_look_up(making, left) {
            Ok(timeout) => (name, timeout),
            Err(why) => return Looked::Refused(why),
        }
    };
    let found = super::look_up(library, making.settings, &name, timeout).await;
    let left = {
        let mut held = lock(holder);
        let seat = held.seat();
        seat.looked_up(making, &name, &found);
        (LOOKUPS_PER_QUESTION - making.looked - 1).min(seat.lookups_left())
    };
    Looked::Made(lookup::message(
        &name,
        &found,
        making.request.question,
        left,
    ))
}

impl Seat {
    /// Files the report `decision` carried ([`crate::reports`]), with what
    /// only the seat may say of it: the question as the model was shown it
    /// and the answer read against it, and its quotes checked against the
    /// card text the seat told the model (the narrator's, and the build's
    /// own text of a card it looked up). Written to the reports file in the
    /// settings' reports directory at once ([`Settings::reports`]), so it
    /// outlives a bug report the gateway never takes, and kept for the
    /// bridge.
    fn file_report(&mut self, report: &Report, decision: &Decision, at: &Making) {
        let request = at.request;
        let pending = crate::scripted::kind(&request.pending);
        let checked = reports::Checked::new(report, pending, |name| {
            let mut told = at.narrator.told_texts(name);
            if self.looked_up.contains(&lookup::fold(name)) {
                told.extend(lookup::compiled(name));
            }
            told
        });
        let filed = reports::Filed {
            source: reports::Source::LlmSeat,
            game_id: request.context.game_id.clone(),
            seat: request.context.seat.get(),
            question: request.question,
            model: at.settings.model.clone(),
            profile: at.settings.profile.clone(),
            report: report.clone(),
            checked,
            moment: reports::Moment::of(request, at.menu, decision),
            lookups: at.looked,
            unix: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_secs()),
            seen: reports::seen_in(report, &request.view),
            log_lines: request
                .log
                .from
                .saturating_add(u32::try_from(request.log.entries.len()).unwrap_or(u32::MAX)),
        };
        let noted = json!({
            "question": request.question,
            "turn": request.view.turn,
            "report": report.observed,
            "kind": filed.checked.kind.as_str(),
            "dedupe_key": filed.checked.dedupe_key,
        });
        if self.reports.file(filed, at.settings.reports.as_deref()) {
            self.transcript.write_value(&noted);
            self.transcript.flush();
        }
    }

    /// How long a lookup's sources may each take, or why no lookup may be
    /// made now: the question's or the game's lookups are spent, or too
    /// little of the question's time is `left`.
    fn may_look_up(&self, at: &Making, left: Duration) -> Result<Duration, String> {
        let question = at.request.question;
        if at.looked >= LOOKUPS_PER_QUESTION {
            return Err(format!(
                "no lookups left for this question ({LOOKUPS_PER_QUESTION} made); answer \
                 q{question} now"
            ));
        }
        if self.lookups >= LOOKUPS_PER_GAME {
            return Err(format!(
                "no lookups left in this game ({LOOKUPS_PER_GAME} made); answer q{question} now"
            ));
        }
        let spare = left.saturating_sub(RETRY_FLOOR);
        if spare < lookup::LEAST {
            return Err(format!("no time left for a lookup; answer q{question} now"));
        }
        Ok((spare / 4).min(lookup::TIMEOUT))
    }

    /// Counts a lookup of `name` that found `found`, and writes it to the
    /// mind's transcript, the text the model is told included.
    fn looked_up(&mut self, at: &Making, name: &str, found: &lookup::Found) {
        self.lookups += 1;
        if found.sources.contains(&lookup::source::BAYLEE) {
            self.looked_up.insert(lookup::fold(name));
        }
        self.transcript.write_value(&json!({
            "question": at.request.question,
            "turn": at.request.view.turn,
            "lookup": {
                "name": name,
                "sources": found.sources,
                "text": found.text,
                "of_question": at.looked + 1,
                "of_game": self.lookups,
            },
        }));
        self.transcript.flush();
    }

    /// Writes to the mind's transcript that the lookup of `name` an
    /// answer asked for beside answering was not made: the answer is taken.
    fn passed_over(&mut self, at: &Making, name: &str) {
        self.transcript.write_value(&json!({
            "question": at.request.question,
            "turn": at.request.view.turn,
            "lookup_passed_over": {
                "name": name,
                "why": "the answer also answered the question",
            },
        }));
        self.transcript.flush();
    }

    /// The lookups this seat has left in its game.
    const fn lookups_left(&self) -> u32 {
        LOOKUPS_PER_GAME.saturating_sub(self.lookups)
    }

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
            lookups: 0,
            looked_up: BTreeSet::new(),
            reports: reports::Book::default(),
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
        if let Some(held) = request.held {
            self.notes.push(held_line(held, request));
        }
        let (action, label) = self.follow(request)?;
        self.last_by_model = false;
        let note = json!({"plan": label}).to_string();
        Some(self.answer(action, note, Duration::ZERO, None, None))
    }

    /// The seat's answer: `action`, with the stops the seat holds and the
    /// steps of its plan left; `orders` for an answer of the model's own.
    pub(crate) fn answer(
        &self,
        action: PlayerAction,
        note: String,
        model_time: Duration,
        thinking: Option<String>,
        orders: Option<Orders>,
    ) -> Answer {
        Answer {
            action,
            model_time,
            note: Some(note),
            thinking,
            stops: self.stops.clone().map(Box::new),
            orders,
            planned: self.queue.as_ref().map_or(0, |queue| queue.steps.len()),
        }
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

    /// The stops the model set, for the header: state it chose.
    pub(crate) fn stops_summary(&self) -> Option<String> {
        self.stops.as_ref().map(|stops| {
            format!(
                "stops: mine [{}], theirs [{}]",
                stops.mine.join(", "),
                stops.theirs.join(", ")
            )
        })
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
        if resolved.board_full {
            self.narrator.tell_whole_board();
        }
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

/// What the model's `until` did since its last answer, in one line.
fn held_line(held: Held, request: &Request) -> String {
    let view = &request.view;
    let table = Table::new(view, &request.context);
    let windows = match held.windows {
        1 => "1 window".to_string(),
        n => format!("{n} windows"),
    };
    let head = format!("until {} held through {windows}", held.until.as_str());
    let Some(why) = held.woke else {
        return format!("{head} and ended where you said.");
    };
    let because = match why {
        Why::OpposingStack => view
            .stack
            .iter()
            .rev()
            .find(|o| !table.ally(o.controller))
            .map_or_else(
                || "something of an opponent's is on the stack".to_string(),
                |o| {
                    format!(
                        "{}'s {} is on the stack",
                        Table::player_id(o.controller),
                        table.named(o.id)
                    )
                },
            ),
        Why::Declaration => "there is an attack or a block to declare".into(),
        Why::Owed => "you owe a payment".into(),
        Why::Cleanup => "a cleanup window opened".into(),
        Why::Question | Why::NotAsked => "the table asks you something".into(),
        Why::RailStop => "a window you stop at came".into(),
    };
    format!("{head}; woke because {because}.")
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
