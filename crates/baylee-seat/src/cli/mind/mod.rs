//! The CLI as a seat's mind: asking it, reading its answer, cooling down.

use super::{
    Answer, Arc, BTreeMap, CliMind, CliSeat, Command, Conversation, Cooldown, Disclosure, Duration,
    GAME_DATA, GameContext, Gone, Instant, LOST, Launch, Limits, MAX_COOLDOWN, Mind, MindError,
    Mutex, Outcome, PROBE, PlayerAction, Prepared, Queue, RETRY_FLOOR, Reader, Readiness, Reply,
    Request, STALE_STORE, Seat, Seats, Session, SessionDir, Settings, Stdio, Store, Tally,
    Thinking, Usage, Value, Waiter, Weak, Wire, collect, decision, detach, json, lock, mpsc,
    narrator, oneshot, prompt, read, scrub, sweep_stores, write,
};
use crate::llm::seatstate::{Holds, Looked, Making, report_and_look_up};
use crate::llm::time_left;
use crate::lookup::Library;
use crate::reports::Filed;

impl Holds for CliSeat {
    fn seat(&mut self) -> &mut Seat {
        &mut self.seat
    }
}

mod process;

impl CliMind {
    /// A mind that plays with `settings` through `launch`'s tool, its
    /// processes living by `limits`.
    #[must_use]
    pub fn new(settings: Settings, launch: Launch, limits: Limits) -> Self {
        let tally = Tally {
            calls_cap: settings.spend_calls,
            ..Tally::default()
        };
        if launch.keeps_conversations() {
            let swept = sweep_stores(&std::env::temp_dir(), STALE_STORE);
            if swept > 0 {
                tracing::info!(swept, "removed stale conversation stores");
            }
        }
        Self {
            system: format!(
                "{}{}{GAME_DATA}",
                prompt::system(settings.extras()),
                prompt::JSON_MODE
            ),
            settings,
            launch,
            seats: Arc::new(Mutex::new(BTreeMap::new())),
            tally: Arc::new(Mutex::new(tally)),
            cooldown: Mutex::new(Cooldown {
                until: None,
                next: limits.cooldown,
            }),
            limits,
            locked_out: Arc::new(Mutex::new(None)),
            warned: std::sync::atomic::AtomicBool::new(false),
            library: Library::shared(),
        }
    }

    /// What the mind has spent so far, shared: it keeps counting while the
    /// mind plays.
    #[must_use]
    pub fn tally(&self) -> Arc<Mutex<Tally>> {
        Arc::clone(&self.tally)
    }

    /// The settings it plays with.
    #[must_use]
    pub const fn settings(&self) -> &Settings {
        &self.settings
    }

    pub(super) fn tool(&self) -> &'static str {
        self.launch.dialect.tool().name()
    }

    pub(super) fn seat(&self, context: &GameContext) -> Arc<Mutex<CliSeat>> {
        let key = (context.game_id.clone(), context.seat.get());
        let mut seats = lock(&self.seats);
        Arc::clone(seats.entry(key).or_insert_with(|| {
            Arc::new(Mutex::new(CliSeat {
                seat: Seat::new(
                    context,
                    self.settings.transcripts.as_deref(),
                    self.settings.style(),
                ),
                session: None,
                lost: false,
                conversation: None,
            }))
        }))
    }

    /// Answers `request`: from the plan or the hint where they fit, else by
    /// asking the tool.
    pub(super) async fn think(&self, request: &Request) -> Result<Answer, MindError> {
        let started = Instant::now();
        if let Some(why) = lock(&self.locked_out).clone() {
            return Err(MindError::Unavailable(why));
        }
        self.reap(&request.context);
        let seat = self.seat(&request.context);
        let mut prepared = match self.start(&seat, request) {
            Ok(prepared) => prepared,
            Err(done) => return done,
        };
        let mut text = prepared.text.clone();
        // Answers that could not be taken: the second is the last. A lookup
        // asks the question again without counting among them.
        let mut refused = 0;
        let mut looked = 0;
        loop {
            let left = time_left(request, started);
            if left.is_zero() {
                return Err(MindError::Declined("no time left to ask the model".into()));
            }
            let sent = Instant::now();
            let reply =
                match tokio::time::timeout(left, self.ask(&seat, &text, request.question)?).await {
                    Ok(Ok(reply)) => reply,
                    Ok(Err(_)) => Reply::Gone(Gone::Ended),
                    Err(_) => {
                        // Hung: killed now, and the next question starts again.
                        Self::note_conversation(&mut lock(&seat));
                        if let Some(session) = Self::lose(&seat) {
                            session.end(Duration::ZERO);
                        }
                        let error =
                            MindError::Unavailable(format!("no reply within {} s", left.as_secs()));
                        Self::record(&seat, request, &text, None, Some(&error));
                        return Err(error);
                    }
                };
            // Whatever it named is the conversation's, to resume or to
            // remove.
            Self::note_conversation(&mut lock(&seat));
            let (value, usage, said) = match reply {
                Reply::Gone(gone)
                    if prepared.resumed
                        && (matches!(gone, Gone::Strayed)
                            || (matches!(gone, Gone::Ended) && !Self::heard(&seat))) =>
                {
                    // The tool could not go on with the conversation (it
                    // was not found, not read, or another was begun): it
                    // begins again, for this very question, from the start.
                    prepared = self
                        .afresh(&seat, request)
                        .map_err(MindError::Unavailable)?;
                    text = prepared.text.clone();
                    refused = 0;
                    continue;
                }
                Reply::Outcome(Outcome::Answer {
                    value,
                    text: said,
                    usage,
                }) => {
                    self.cooled();
                    Self::record(&seat, request, &text, Some((&value, &said, usage)), None);
                    (value, usage, said)
                }
                Reply::Outcome(Outcome::RateLimited { why, lifts_in }) => {
                    self.cool(lifts_in);
                    return Err(limited(&why, lifts_in));
                }
                Reply::Outcome(Outcome::Failed(why)) => {
                    return Err(MindError::Unavailable(scrub(&why, None)));
                }
                Reply::Gone(gone) => return Err(self.gone(&seat, gone).await),
            };
            let taken = decision(value.as_ref());
            let making = Making::new(request, &self.settings, looked);
            let left = time_left(request, started);
            let extras =
                report_and_look_up(&seat, &self.library, &making, taken.as_ref().ok(), left);
            // A lookup alone asks the same question again with the card's
            // text; one beside an answer, or one that may not be made, is
            // passed over, and the answer is taken if it answers.
            let no_lookup = match extras.await {
                Looked::Made(message) => {
                    looked += 1;
                    text = self.reask(&seat, request, &mut prepared, message)?;
                    continue;
                }
                Looked::Refused(why) => Some(why),
                Looked::Nothing => None,
            };
            match taken.and_then(|decision| read(decision, &prepared.menu)) {
                Ok(read) => {
                    return Self::commit(&seat, request, prepared, read, usage, sent, said.clone());
                }
                Err(why) => {
                    let why = no_lookup.unwrap_or(why);
                    refused += 1;
                    if refused >= 2 || time_left(request, started) < RETRY_FLOOR {
                        return Err(MindError::Declined(format!(
                            "the model's answer could not be read: {why}"
                        )));
                    }
                    text = self.again(&seat, request, &mut prepared, &why)?;
                }
            }
        }
    }

    /// The message that asks `request` again after its answer could not be
    /// read for `why` ([`Self::reask`]).
    pub(super) fn again(
        &self,
        seat: &Arc<Mutex<CliSeat>>,
        request: &Request,
        prepared: &mut Prepared,
        why: &str,
    ) -> Result<String, MindError> {
        let correction = format!(
            "That answer could not be taken: {why}. Answer q{} again.",
            request.question
        );
        self.reask(seat, request, prepared, correction)
    }

    /// The message that asks `request` again with `message` (why the last
    /// answer could not be taken, or a lookup's card). A one-shot tool's
    /// goes to a process of its own: one that goes on with the
    /// conversation hears only `message`, one that has heard nothing yet
    /// the whole question again.
    pub(super) fn reask(
        &self,
        seat: &Arc<Mutex<CliSeat>>,
        request: &Request,
        prepared: &mut Prepared,
        message: String,
    ) -> Result<String, MindError> {
        if !self.launch.dialect.one_shot() {
            return Ok(message);
        }
        prepared.resumed = self
            .respawn(seat, &request.context)
            .map_err(MindError::Unavailable)?;
        Ok(if prepared.resumed {
            message
        } else {
            format!("{}\n\n{message}", prepared.text)
        })
    }

    /// What is done before the tool is asked: the log heard, a refusal or a
    /// late answer noted, the plan or the hint followed, the budget and the
    /// cooldown checked, the message told, and a process started for a
    /// new conversation. `Err` is the answer when nothing is sent.
    #[allow(clippy::result_large_err)]
    pub(super) fn start(
        &self,
        seat: &Arc<Mutex<CliSeat>>,
        request: &Request,
    ) -> Result<Prepared, Result<Answer, MindError>> {
        let mut state = lock(seat);
        // With no tool results, a refusal always goes into the notes.
        if let Some(answer) = state.seat.begin(request, |_| false) {
            return Err(Ok(answer));
        }
        if let Some(why) = lock(&self.tally).spent_under(&self.settings) {
            return Err(Err(MindError::Declined(why)));
        }
        if let Some(left) = self.cooling() {
            return Err(Err(MindError::Limited {
                why: format!("{} s left of the cooldown", left.as_secs().max(1)),
                lifts_in: Some(left),
            }));
        }
        state.seat.asked += 1;
        let dialect = &self.launch.dialect;
        // A process of a tool that answers one message is done with once it
        // answered: each question starts its own, and that is no loss.
        // Nor is one ended idle, or killed for hanging: no conversation of
        // it was going on, unless the tool resumes one, whose loss is the
        // conversation's own.
        if dialect.one_shot() {
            if let Some(done) = state.session.take() {
                done.end(self.limits.grace);
            }
            if !dialect.resumes() {
                state.lost = false;
            }
        }
        // A process that ended since the last message (between turns, say)
        // is begun again for this question rather than costing it.
        if state.session.as_mut().is_some_and(Session::died) {
            if let Some(dead) = state.session.take() {
                dead.end(Duration::ZERO);
            }
            state.lost = true;
        }
        // One conversation across turns: only its size ends it, and, kept
        // on disk, its idleness, as a process's would.
        let tokens = self.settings.conversation_tokens;
        let fresh = if dialect.resumes() {
            let idle = self.limits.idle;
            let goes_on = state.conversation.as_ref().is_some_and(|going| {
                going.kept() && going.used.elapsed() < idle && going.sent.div_ceil(3) <= tokens
            });
            if goes_on {
                state.lost = false;
            } else if let Some(over) = state.conversation.take() {
                // Over by its size is no loss; idle, or its files gone, is.
                state.lost = over.id.is_some() && (over.used.elapsed() >= idle || !over.kept());
            }
            !goes_on
        } else {
            state
                .session
                .as_ref()
                .is_none_or(|session| session.sent.div_ceil(3) > tokens)
        };
        let mut prepared = Self::tell(&state, request, fresh);
        if fresh || state.session.is_none() {
            prepared.resumed = self
                .open(&mut state, seat, &request.context, fresh)
                .map_err(|why| Err(MindError::Unavailable(why)))?;
        }
        Ok(prepared)
    }

    /// The message for `request`: what is new since the last, or, `fresh`,
    /// a conversation's first, with the prefix, the seat's notes and, after
    /// a loss, that it was lost.
    pub(super) fn tell(state: &CliSeat, request: &Request, fresh: bool) -> Prepared {
        let mut told = state.seat.told(fresh);
        if fresh && state.lost {
            told.insert(0, LOST.into());
        }
        let mut narrator = state.seat.narrator.clone();
        if fresh {
            narrator.forget_cards();
        }
        let stops_summary = state.seat.stops_summary();
        let wake = narrator.wake(request, &told, stops_summary.as_deref());
        let text = if fresh {
            format!("{}\n\n{}", narrator.prefix(&request.context), wake.text)
        } else {
            wake.text
        };
        Prepared {
            menu: wake.menu,
            narrator,
            text,
            asked: state.seat.asked,
            resumed: false,
        }
    }

    /// Starts the seat's next process in place of its last one: for a
    /// `fresh` conversation (a tool that keeps its conversations on disk
    /// gets a new store for it, and a start after a loss is counted), else
    /// going on with the seat's conversation where its tool resumes one.
    /// Whether it resumes one.
    pub(super) fn open(
        &self,
        state: &mut CliSeat,
        seat: &Arc<Mutex<CliSeat>>,
        context: &GameContext,
        fresh: bool,
    ) -> Result<bool, String> {
        // A lockout found since the question began starts nothing.
        if let Some(why) = lock(&self.locked_out).clone() {
            return Err(why);
        }
        if let Some(old) = state.session.take() {
            old.end(self.limits.grace);
        }
        if fresh && self.launch.keeps_conversations() {
            let store = Store::new(context.seat.get()).map_err(|e| {
                format!(
                    "{}'s conversation store could not be made: {e}",
                    self.tool()
                )
            })?;
            state.conversation = Some(Conversation {
                id: None,
                sent: 0,
                used: Instant::now(),
                ids: Vec::new(),
                sessions: self
                    .launch
                    .sessions_root()
                    .map(|root| (Arc::clone(&self.launch.dialect), root)),
                counted: Arc::default(),
                store,
            });
        }
        let session = self.spawn(context, Arc::downgrade(seat), state.conversation.as_ref())?;
        let resumed = state
            .conversation
            .as_ref()
            .is_some_and(|going| going.id.is_some());
        state.session = Some(session);
        let mut tally = lock(&self.tally);
        tally.sessions += 1;
        if fresh && std::mem::take(&mut state.lost) {
            tally.restarts += 1;
        }
        Ok(resumed)
    }

    /// Starts a new process for the seat in place of its last one, which a
    /// one-shot tool is done with: a question asked again goes to a
    /// process that has heard nothing, or, where the tool resumes the
    /// conversation, to one that goes on with it. Whether it does.
    pub(super) fn respawn(
        &self,
        seat: &Arc<Mutex<CliSeat>>,
        context: &GameContext,
    ) -> Result<bool, String> {
        let mut state = lock(seat);
        Self::note_conversation(&mut state);
        self.open(&mut state, seat, context, false)
    }

    /// Begins the seat's conversation again for `request`, after its tool
    /// could not resume it: from the start, saying it was lost, counted.
    pub(super) fn afresh(
        &self,
        seat: &Arc<Mutex<CliSeat>>,
        request: &Request,
    ) -> Result<Prepared, String> {
        let mut state = lock(seat);
        if let Some(dead) = state.session.take() {
            dead.end(Duration::ZERO);
        }
        state.conversation = None;
        state.lost = true;
        let prepared = Self::tell(&state, request, true);
        self.open(&mut state, seat, &request.context, true)?;
        Ok(prepared)
    }

    /// Keeps the id the seat's process named its conversation by, for the
    /// next process to resume ([`Dialect::resumes`]) and for the seat to
    /// remove with it: a tool's conversation kept on disk
    /// ([`Launch::keeps_conversations`]); nothing for any other.
    pub(super) fn note_conversation(state: &mut CliSeat) {
        let (Some(session), Some(going)) = (&state.session, &mut state.conversation) else {
            return;
        };
        if let Some(id) = lock(&session.wire).conversation.clone() {
            going.named(id);
        }
    }

    /// Whether the seat's process said anything before it ended.
    pub(super) fn heard(seat: &Mutex<CliSeat>) -> bool {
        lock(seat)
            .session
            .as_ref()
            .is_some_and(|session| lock(&session.queue).heard)
    }

    /// Keeps what the model answered, and turns it into the seat's answer.
    pub(super) fn commit(
        seat: &Mutex<CliSeat>,
        request: &Request,
        prepared: Prepared,
        (resolved, say): (narrator::Resolved, Option<String>),
        usage: Option<Usage>,
        sent: Instant,
        thinking: String,
    ) -> Result<Answer, MindError> {
        let mut state = lock(seat);
        if state.seat.asked != prepared.asked {
            return Err(MindError::Declined("a newer question replaced it".into()));
        }
        let label = resolved.label.clone();
        let orders = crate::wake::Orders {
            until: resolved.until,
            react: resolved.react,
        };
        let action = state
            .seat
            .keep(request, prepared.narrator, resolved, say.as_deref());
        let took = sent.elapsed();
        let note = json!({
            "chose": label,
            "say": say,
            "tokens": usage,
            "ms": u64::try_from(took.as_millis()).unwrap_or(u64::MAX),
        });
        Ok(state
            .seat
            .answer(action, note.to_string(), took, Some(thinking), Some(orders)))
    }

    /// The seat's process, taken from it, its conversation noted as lost.
    pub(super) fn lose(seat: &Mutex<CliSeat>) -> Option<Session> {
        let mut state = lock(seat);
        state.lost = true;
        state.conversation = None;
        state.session.take()
    }

    /// Ends a process that stopped answering, and says why: its exit and
    /// its last words, or the lockdown a process broke, which takes the
    /// mind off the table for good (its reader has done so already).
    pub(super) async fn gone(&self, seat: &Mutex<CliSeat>, gone: Gone) -> MindError {
        if let Gone::Limited { why, lifts_in } = gone {
            // Ended, not lost: its conversation never began.
            if let Some(session) = lock(seat).session.take() {
                session.end(Duration::ZERO);
            }
            if let Some(why) = lock(&self.locked_out).clone() {
                return MindError::Unavailable(why);
            }
            self.cool(lifts_in);
            return limited(&why, lifts_in);
        }
        let session = Self::lose(seat);
        if let Gone::Refused(why) = &gone {
            lock_out(&self.locked_out, Some(&self.seats), why);
        }
        if let Some(why) = lock(&self.locked_out).clone() {
            if let Some(session) = session {
                session.end(Duration::ZERO);
            }
            return MindError::Unavailable(why);
        }
        let tool = self.tool();
        let Some(mut session) = session else {
            return MindError::Unavailable(format!("the {tool} process ended"));
        };
        let status = tokio::time::timeout(Duration::from_millis(500), session.child.wait())
            .await
            .ok()
            .and_then(Result::ok)
            .map(|status| format!(" ({status})"))
            .unwrap_or_default();
        let words = session
            .last_words()
            .map(|words| format!(": {words}"))
            .unwrap_or_default();
        session.end(Duration::ZERO);
        MindError::Unavailable(format!("the {tool} process ended{status}{words}"))
    }

    /// Ends the processes idle past [`Limits::idle`], and, when the seat of
    /// `context` has none, the least recently used idle ones beyond
    /// [`Limits::max_sessions`] less one, so it may start its own.
    pub(super) fn reap(&self, context: &GameContext) {
        let me = (context.game_id.clone(), context.seat.get());
        let seats: Vec<_> = lock(&self.seats)
            .iter()
            .map(|(key, seat)| (key.clone(), Arc::clone(seat)))
            .collect();
        let now = Instant::now();
        let (mut idle, mut live, mut mine) = (Vec::new(), 0, false);
        for (key, seat) in seats {
            let mut state = lock(&seat);
            let Some(session) = &state.session else {
                continue;
            };
            let busy = !lock(&session.queue).waiting.is_empty();
            let used = session.used;
            if !busy && now.duration_since(used) >= self.limits.idle {
                // A one-shot process holds no conversation: ending it loses
                // nothing (one kept on disk is over by its own idleness).
                self.retire(&mut state);
                continue;
            }
            live += 1;
            if key == me {
                mine = true;
            } else if !busy {
                drop(state);
                idle.push((used, seat));
            }
        }
        if mine {
            return;
        }
        idle.sort_by_key(|(used, _)| *used);
        for (_, seat) in idle {
            if live < self.limits.max_sessions {
                break;
            }
            let mut state = lock(&seat);
            if self.retire(&mut state) {
                live -= 1;
            }
        }
    }

    /// Ends the seat's process because it is idle or its place is wanted,
    /// with its grace. The conversation it held is lost, unless the tool
    /// answers one message a process; one it kept on disk goes with it
    /// where no later process resumes it. Whether there was one.
    pub(super) fn retire(&self, state: &mut CliSeat) -> bool {
        Self::note_conversation(state);
        let Some(session) = state.session.take() else {
            return false;
        };
        state.lost |= !self.launch.dialect.one_shot();
        if !self.launch.dialect.resumes() {
            state.conversation = None;
        }
        session.end(self.limits.grace);
        true
    }

    /// Cools the mind down after a rate limit: until it lifts, where the
    /// tool said so believably ([`believed`]), else for the next step of
    /// the doubling cooldown.
    pub(super) fn cool(&self, lifts_in: Option<Duration>) {
        let lifts_in = believed(lifts_in);
        let mut cooldown = lock(&self.cooldown);
        let wait = lifts_in.unwrap_or(cooldown.next);
        cooldown.until = Some(Instant::now() + wait);
        if lifts_in.is_none() {
            cooldown.next = (cooldown.next * 2).min(MAX_COOLDOWN);
        }
    }

    /// An answer came: the next rate limit cools from the start again.
    pub(super) fn cooled(&self) {
        let mut cooldown = lock(&self.cooldown);
        cooldown.until = None;
        cooldown.next = self.limits.cooldown;
    }

    /// How long the mind still cools down, if it does.
    pub(super) fn cooling(&self) -> Option<Duration> {
        lock(&self.cooldown)
            .until
            .and_then(|until| until.checked_duration_since(Instant::now()))
            .filter(|left| !left.is_zero())
    }

    /// Whether the tool is signed in, by its login check, which calls no
    /// model: run as a session's process is, and given ten seconds.
    pub(super) async fn probe(&self) -> bool {
        self.check_login().await.is_ok()
    }

    /// [`Self::probe`], saying why not, in a sentence for the chair's card.
    pub(super) async fn check_login(&self) -> Result<(), String> {
        let tool = self.launch.dialect.tool().name();
        let Some(args) = self.launch.dialect.probe_args() else {
            return Ok(());
        };
        let dir = SessionDir::new(0)
            .map_err(|_| format!("{tool}'s login check found no place to run"))?;
        // A tool that keeps conversations keeps the check's in its own
        // directory too, never in the user's.
        let env = self
            .launch
            .env(&dir.tmp(), &dir.support(), Some(&dir.tmp()))
            .map_err(|_| format!("{tool}'s login check could not be given its variables"))?;
        let mut command = Command::new(&self.launch.program);
        command
            .args(args)
            .env_clear()
            .envs(env)
            .current_dir(dir.work())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if self.launch.dialect.detaches() {
            detach(&mut command);
        }
        let child = command
            .spawn()
            .map_err(|e| format!("{tool} did not start for its login check: {e}"))?;
        match tokio::time::timeout(PROBE, child.wait_with_output()).await {
            Ok(Ok(output))
                if output.status.success()
                    && self.launch.dialect.probe_ok(&output.stdout, &output.stderr) =>
            {
                Ok(())
            }
            Ok(Ok(_)) => Err(format!(
                "{tool} is not signed in: sign in to it on this machine first"
            )),
            Ok(Err(e)) => Err(format!("{tool}'s login check failed: {e}")),
            Err(_) => Err(format!(
                "{tool} did not answer its login check within {} s",
                PROBE.as_secs()
            )),
        }
    }

    /// One line of the mind's own transcript: the message, the reply.
    pub(super) fn record(
        seat: &Mutex<CliSeat>,
        request: &Request,
        text: &str,
        reply: Option<(&Option<Value>, &str, Option<Usage>)>,
        error: Option<&MindError>,
    ) {
        let mut state = lock(seat);
        let transcript = &mut state.seat.transcript;
        transcript.write_value(&json!({
            "question": request.question,
            "turn": request.view.turn,
            "message": text,
            "reply": reply.map(|(value, said, usage)| json!({
                "answer": value,
                "text": said,
                "usage": usage,
            })),
            "error": error.map(ToString::to_string),
        }));
        transcript.flush();
    }
}

/// The time a tool said its limit lifts in, where the mind believes it:
/// some time, and no more than [`MAX_COOLDOWN`]. None, a past time or a
/// longer one is as if it said nothing, and the doubling cooldown holds.
pub(super) fn believed(lifts_in: Option<Duration>) -> Option<Duration> {
    lifts_in.filter(|wait| !wait.is_zero() && *wait <= MAX_COOLDOWN)
}

/// A rate limit or a spent quota the tool reported, in its words: the
/// mind is limited ([`MindError::Limited`]), as a model behind an API is
/// when its provider says so. A fallback profile takes the question over
/// where one stands by; with none, the house plays the rest of the game
/// (`docs/llm-seat.md` §"A CLI as the model", §"A fallback profile").
fn limited(why: &str, lifts_in: Option<Duration>) -> MindError {
    MindError::Limited {
        why: scrub(why, None),
        lifts_in,
    }
}

/// Takes the mind off the table for good, for `why` (the first reason
/// stays): every process in `seats` is killed now, each seat's conversation
/// noted as lost.
pub(super) fn lock_out(locked_out: &Mutex<Option<String>>, seats: Option<&Seats>, why: &str) {
    lock(locked_out).get_or_insert_with(|| why.to_string());
    let Some(seats) = seats else {
        return;
    };
    let seats: Vec<_> = lock(seats).values().cloned().collect();
    for seat in seats {
        let mut state = lock(&seat);
        if let Some(session) = state.session.take() {
            state.lost = true;
            session.end(Duration::ZERO);
        }
    }
}

impl Mind for CliMind {
    fn decide(&self, request: Request) -> Thinking<'_> {
        Box::pin(async move {
            let answer = self.think(&request).await;
            if let Ok(Answer {
                action: PlayerAction::CastSpell { card },
                ..
            }) = &answer
            {
                lock(&self.seat(&request.context)).seat.casting = Some(*card);
            }
            answer
        })
    }

    fn reports(&self, game: &str, seat: u8) -> Vec<Filed> {
        let state = lock(&self.seats).get(&(game.to_owned(), seat)).cloned();
        state
            .map(|state| lock(&state).seat.reports.take())
            .unwrap_or_default()
    }

    fn disclosure(&self) -> Disclosure {
        Disclosure::Llm
    }

    fn ready(&self) -> Readiness<'_> {
        Box::pin(async move {
            if lock(&self.locked_out).is_some() || self.cooling().is_some() {
                return false;
            }
            self.probe().await
        })
    }

    fn check(&self) -> crate::mind::Checked<'_> {
        Box::pin(async move {
            if lock(&self.locked_out).is_some() {
                return Err("the tool locked this seat out".to_string());
            }
            if let Some(left) = self.cooling() {
                return Err(format!(
                    "the tool is cooling down for {} s more",
                    left.as_secs()
                ));
            }
            self.check_login().await
        })
    }
}
