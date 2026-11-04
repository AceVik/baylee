//! The fill as a state machine: told the time, what the player wants and what
//! each request came back with, it says what to do next.

use super::meter::Meter;
use super::retry::Backoff;
use super::{FREE_FLOOR, FillChoices, Gate, Picture, Reach, Round, Turn};
use std::collections::VecDeque;
use std::time::Duration;

/// How long after it is switched on the fill starts: a launch is followed by a
/// front door, a lobby and their pictures, and all of those are wanted.
const SETTLE: Duration = Duration::from_secs(5);

/// How often the cache's real size and the disk's free room are read again,
/// on top of what the fill counts itself: wanted pictures land there too.
const MEASURE_EVERY: Duration = Duration::from_secs(60);

/// How many 404s in a row mean the server has no pictures rather than this
/// one: a wrong address answers 404 for every printing.
const MISSING_RUN: u32 = 16;

/// What the client wants of the fill, as it stands this frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Steering {
    /// The player's settings.
    pub choices: FillChoices,
    /// A game or a replay is open: it should not lose a byte or a frame.
    pub playing: bool,
    /// The player pressed Pause.
    pub held: bool,
}

/// What kept going wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trouble {
    /// No connection, or no answer.
    Unreachable,
    /// A server said it was too busy (429 or 503).
    Busy,
    /// A server failed (another 5xx).
    Server(u16),
}

/// Why the fill stopped. It stays stopped until the player says otherwise or
/// the client starts again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// This device has no cache directory.
    NoCache,
    /// A picture could not be written, so downloading more would be waste.
    CannotWrite,
    /// The cache reached the budget.
    Budget,
    /// The disk reached [`FREE_FLOOR`].
    DiskFull,
    /// [`super::MAX_FAILURES`] failures in a row.
    Failing(Trouble),
    /// A server refused outright (a 4xx that is not a busy one, a redirect),
    /// or answered 404 for picture after picture.
    Refused(u16),
}

/// Why the fill is resting. It goes on by itself once the reason is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// The player pressed Pause.
    Player,
    /// A game is on.
    Match,
}

/// What the fill is doing, as a player is told.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    /// Switched off.
    #[default]
    Off,
    /// Resting.
    Paused(Why),
    /// Working out what is missing.
    Checking,
    /// Downloading.
    Running,
    /// Staying away for this long, because a server said so or failed.
    Cooling(Duration),
    /// Everything planned is on disk.
    Finished,
    /// Stopped.
    Stopped(Stop),
}

/// One picture to fetch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Job {
    /// The plan it belongs to.
    epoch: u64,
    /// Its place in that plan.
    slot: usize,
    /// The URL.
    pub url: String,
}

/// What to do next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Next {
    /// List the cache and hand [`Fill::planned`] what is there.
    Check {
        /// The plan this will be.
        epoch: u64,
        /// How much of a card.
        reach: Reach,
    },
    /// Read the cache's size and the disk's room for [`Fill::measured`].
    Measure,
    /// Fetch this, and hand the answer to [`Fill::finished`].
    Fetch(Job),
    /// Empty the cache, and say so with [`Fill::wiped`].
    Wipe,
    /// Ask again after this long.
    Wait(Duration),
    /// Nothing to do until something changes.
    Idle,
}

/// What became of one request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// The picture was downloaded and stored: this many bytes.
    Stored(u64),
    /// It was on disk by the time its turn came (a draw got there first).
    Already,
    /// The server has no such picture.
    Missing,
    /// It stopped for a wanted picture, and goes back in the queue.
    GaveWay,
    /// 429 or 503, with what `Retry-After` said.
    Busy(Option<Duration>),
    /// Another 5xx, with what `Retry-After` said.
    Server(u16, Option<Duration>),
    /// No answer at all.
    Unreachable,
    /// Refused outright, by this status.
    Refused(u16),
    /// It arrived and could not be written.
    CannotWrite,
}

/// What the settings and the indicator show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// What the fill is doing.
    pub status: Status,
    /// The round it is in.
    pub round: Option<Round>,
    /// How many rounds the plan has.
    pub rounds: u8,
    /// Cards with this round's picture on disk.
    pub cards_done: u32,
    /// Cards in this round.
    pub cards_total: u32,
    /// Pictures on disk, of every round.
    pub pictures_done: u32,
    /// Pictures planned, of every round.
    pub pictures_total: u32,
    /// Pictures the server does not have, among the done.
    pub missing: u32,
    /// Bytes downloaded since the plan was made.
    pub bytes: u64,
    /// How large the cache is, once it has been read.
    pub cached: Option<u64>,
    /// How large the fill may let it grow.
    pub budget: u64,
    /// How long the fill has been at work since the plan was made, its rests
    /// not counted.
    pub elapsed: Duration,
    /// Bytes a second, just now, while it is at work.
    pub speed: Option<u64>,
    /// How much longer at this pace, while it is at work.
    pub left: Option<Duration>,
}

/// Where a plan stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// No plan yet.
    Unplanned,
    /// The cache is being listed.
    Checking,
    /// Fetching.
    Filling,
    /// Nothing left.
    Finished,
    /// Given up.
    Stopped(Stop),
}

/// The fill.
#[derive(Debug)]
pub struct Fill {
    gate: Gate,
    steering: Steering,
    /// Which plan is in force: an answer to an older one changes nothing.
    epoch: u64,
    phase: Phase,
    pictures: Vec<Picture>,
    /// What is still to fetch, in order.
    todo: VecDeque<usize>,
    /// Requests of this plan in flight.
    in_flight: usize,
    /// Done and total cards, by round.
    cards: [(u32, u32); 2],
    pictures_done: u32,
    missing: u32,
    /// 404s since the last picture.
    missing_run: u32,
    bytes: u64,
    /// How large the cache is, as last read and counted since.
    cached: Option<u64>,
    /// How much room the disk has, as last read and counted since.
    free: Option<u64>,
    elapsed: Duration,
    /// When [`Self::elapsed`] last moved.
    ticked: Duration,
    meter: Meter,
    backoff: Backoff,
    /// Nothing is started before this: the settling at the start, and the
    /// back-off.
    not_before: Duration,
    measure_at: Duration,
    /// The player asked for the cache to be emptied.
    wipe: bool,
    /// The first reading of the cache has been asked for.
    looked: bool,
}

impl Default for Fill {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl Fill {
    /// A fill with nothing to do yet, its waits stretched by `spread`
    /// ([`Backoff::new`]). Off until [`Self::steer`] says otherwise.
    #[must_use]
    pub fn new(spread: f32) -> Self {
        Self {
            gate: Gate::default(),
            steering: Steering {
                choices: FillChoices {
                    on: false,
                    ..FillChoices::default()
                },
                ..Steering::default()
            },
            epoch: 0,
            phase: Phase::Unplanned,
            pictures: Vec::new(),
            todo: VecDeque::new(),
            in_flight: 0,
            cards: [(0, 0); 2],
            pictures_done: 0,
            missing: 0,
            missing_run: 0,
            bytes: 0,
            cached: None,
            free: None,
            elapsed: Duration::ZERO,
            ticked: Duration::ZERO,
            meter: Meter::default(),
            backoff: Backoff::new(spread),
            not_before: Duration::ZERO,
            measure_at: Duration::ZERO,
            wipe: false,
            looked: false,
        }
    }

    /// The gate, for a wanted picture.
    pub const fn gate(&mut self) -> &mut Gate {
        &mut self.gate
    }

    /// Whether a wanted picture is in flight.
    #[must_use]
    pub const fn yielding(&self) -> bool {
        self.gate.yielding()
    }

    fn at_work(&self) -> bool {
        let steering = &self.steering;
        self.phase == Phase::Filling && steering.choices.on && !steering.held && !steering.playing
    }

    /// Moves [`Self::elapsed`] on, if the fill was at work.
    fn tick(&mut self, now: Duration) {
        let span = now.saturating_sub(self.ticked);
        self.ticked = now;
        if self.at_work() {
            self.elapsed += span;
        }
    }

    /// Takes in what the client wants. A new reach is a new plan, and so is
    /// switching it on: the disk may have moved on while it was off.
    pub fn steer(&mut self, steering: Steering, now: Duration) {
        if self.steering == steering {
            return;
        }
        self.tick(now);
        let old = std::mem::replace(&mut self.steering, steering);
        let new = &self.steering;
        let replan = old.choices.reach != new.choices.reach || (!old.choices.on && new.choices.on);
        let room =
            self.phase == Phase::Stopped(Stop::Budget) && new.choices.budget > old.choices.budget;
        if (new.playing && !old.playing) || (new.held && !old.held) {
            self.meter.clear();
        }
        if replan || room {
            self.restart(now + SETTLE);
        }
    }

    /// Starts over: a new plan, no sooner than `not_before`. Not where there
    /// is no cache: no worker was started, and none will answer.
    fn restart(&mut self, not_before: Duration) {
        if self.phase == Phase::Stopped(Stop::NoCache) {
            return;
        }
        self.epoch += 1;
        self.phase = Phase::Unplanned;
        self.pictures = Vec::new();
        self.todo = VecDeque::new();
        self.in_flight = 0;
        self.cards = [(0, 0); 2];
        self.pictures_done = 0;
        self.missing = 0;
        self.missing_run = 0;
        self.bytes = 0;
        self.elapsed = Duration::ZERO;
        self.meter.clear();
        self.backoff.success();
        self.not_before = not_before;
    }

    /// The player said to try again after a stop.
    pub fn retry(&mut self, now: Duration) {
        if matches!(self.phase, Phase::Stopped(_)) {
            self.restart(now);
        }
    }

    /// The player asked for the cache to be emptied.
    pub const fn empty(&mut self) {
        self.wipe = true;
    }

    /// The cache has been emptied.
    pub fn wiped(&mut self, now: Duration) {
        self.cached = Some(0);
        self.restart(now);
    }

    /// This device has no cache directory: there is nothing to fill.
    pub const fn no_cache(&mut self) {
        self.phase = Phase::Stopped(Stop::NoCache);
    }

    fn stop(&mut self, why: Stop) {
        self.phase = Phase::Stopped(why);
        self.todo.clear();
    }

    /// What stops the fill before the next picture.
    fn over(&self) -> Option<Stop> {
        if self.cached.unwrap_or(0) >= self.steering.choices.budget.bytes() {
            return Some(Stop::Budget);
        }
        self.free
            .filter(|free| *free <= FREE_FLOOR)
            .map(|_| Stop::DiskFull)
    }

    /// What to do next.
    pub fn next(&mut self, now: Duration) -> Next {
        self.tick(now);
        if self.wipe {
            self.wipe = false;
            return Next::Wipe;
        }
        // Once a run, on or off: the settings say how large the cache is
        // whether or not it is being filled. A game goes first, as ever.
        if !self.looked && !self.steering.playing {
            self.looked = true;
            return Next::Measure;
        }
        let steering = &self.steering;
        if !steering.choices.on || steering.held || steering.playing {
            return Next::Idle;
        }
        let parallel = steering.choices.pace.parallel();
        match self.phase {
            Phase::Finished | Phase::Stopped(_) | Phase::Checking => return Next::Idle,
            Phase::Unplanned => {
                if now < self.not_before {
                    return Next::Wait(self.not_before.saturating_sub(now));
                }
                let check = Next::Check {
                    epoch: self.epoch,
                    reach: steering.choices.reach,
                };
                self.phase = Phase::Checking;
                return check;
            }
            Phase::Filling => {}
        }
        if self.todo.is_empty() {
            if self.in_flight == 0 {
                self.phase = Phase::Finished;
            }
            return Next::Idle;
        }
        if now >= self.measure_at {
            self.measure_at = now + MEASURE_EVERY;
            return Next::Measure;
        }
        if let Some(why) = self.over() {
            self.stop(why);
            return Next::Idle;
        }
        if now < self.not_before {
            return Next::Wait(self.not_before.saturating_sub(now));
        }
        match self.gate.fill_turn(now, parallel) {
            Turn::Go => {
                let Some(slot) = self.todo.pop_front() else {
                    self.gate.fill_finished();
                    return Next::Idle;
                };
                self.in_flight += 1;
                Next::Fetch(Job {
                    epoch: self.epoch,
                    slot,
                    url: self.pictures[slot].url.clone(),
                })
            }
            Turn::Wait(wait) => Next::Wait(wait),
        }
    }

    /// The cache has been listed: `pictures` is the plan, `present` says
    /// which of them are on disk, `cached` how large the cache is and `free`
    /// how much room its disk has, where that can be said.
    pub fn planned(
        &mut self,
        epoch: u64,
        pictures: Vec<Picture>,
        present: &[bool],
        cached: u64,
        free: Option<u64>,
        now: Duration,
    ) {
        if epoch != self.epoch || self.phase != Phase::Checking {
            return;
        }
        self.todo = VecDeque::new();
        self.cards = [(0, 0); 2];
        self.pictures_done = 0;
        for (slot, picture) in pictures.iter().enumerate() {
            let there = present.get(slot).copied().unwrap_or(false);
            let round = picture.round.index();
            if picture.card {
                self.cards[round].1 += 1;
            }
            if there {
                self.count_done(picture);
            } else {
                self.todo.push_back(slot);
            }
        }
        self.pictures = pictures;
        self.cached = Some(cached);
        self.free = free;
        self.measure_at = now + MEASURE_EVERY;
        self.ticked = now;
        self.phase = if self.todo.is_empty() {
            Phase::Finished
        } else {
            Phase::Filling
        };
    }

    fn count_done(&mut self, picture: &Picture) {
        self.pictures_done += 1;
        if picture.card {
            self.cards[picture.round.index()].0 += 1;
        }
    }

    /// The cache's size and the disk's room, read again.
    pub const fn measured(&mut self, cached: u64, free: Option<u64>) {
        self.cached = Some(cached);
        self.free = free;
    }

    /// A fill request came back. Whatever plan it was for, its slot in the
    /// gate is free again.
    pub fn finished(&mut self, job: &Job, answer: Answer, now: Duration) {
        self.gate.fill_finished();
        self.tick(now);
        if job.epoch != self.epoch {
            return;
        }
        self.in_flight = self.in_flight.saturating_sub(1);
        match answer {
            Answer::Stored(bytes) => {
                self.bytes += bytes;
                self.cached = Some(self.cached.unwrap_or(0) + bytes);
                self.free = self.free.map(|free| free.saturating_sub(bytes));
                self.meter.note(now, bytes);
                self.done(job.slot);
            }
            Answer::Already => self.done(job.slot),
            Answer::Missing => {
                self.missing += 1;
                self.missing_run += 1;
                let picture = self.pictures[job.slot].clone();
                self.count_done(&picture);
                if self.missing_run >= MISSING_RUN {
                    self.stop(Stop::Refused(404));
                }
            }
            Answer::GaveWay => self.todo.push_front(job.slot),
            Answer::CannotWrite => self.stop(Stop::CannotWrite),
            Answer::Refused(code) => self.stop(Stop::Refused(code)),
            Answer::Busy(_) | Answer::Server(..) | Answer::Unreachable => {
                self.todo.push_front(job.slot);
                self.trouble(answer, now);
            }
        }
    }

    fn done(&mut self, slot: usize) {
        self.backoff.success();
        self.missing_run = 0;
        let picture = self.pictures[slot].clone();
        self.count_done(&picture);
    }

    /// A server failed or asked for a break.
    fn trouble(&mut self, answer: Answer, now: Duration) {
        let (trouble, slow_down, asked) = match answer {
            Answer::Busy(asked) => (Trouble::Busy, true, asked),
            Answer::Server(code, asked) => (Trouble::Server(code), false, asked),
            _ => (Trouble::Unreachable, false, None),
        };
        // The requests of one bad moment all come back within the wait the
        // first one earned, and they are one failure and not three. What a
        // server said is still obeyed.
        if now < self.not_before {
            if let Some(asked) = asked {
                self.not_before = self.not_before.max(now + asked);
            }
            return;
        }
        let wait = self.backoff.failure(slow_down && asked.is_none());
        self.not_before = now + asked.map_or(wait, |asked| wait.max(asked));
        if self.backoff.spent() {
            self.stop(Stop::Failing(trouble));
        }
    }

    /// What the fill is doing now.
    #[must_use]
    pub fn status(&self, now: Duration) -> Status {
        let steering = &self.steering;
        if !steering.choices.on {
            return Status::Off;
        }
        match self.phase {
            Phase::Stopped(why) => return Status::Stopped(why),
            Phase::Finished => return Status::Finished,
            Phase::Unplanned | Phase::Checking | Phase::Filling => {}
        }
        if steering.held {
            return Status::Paused(Why::Player);
        }
        if steering.playing {
            return Status::Paused(Why::Match);
        }
        if self.phase != Phase::Filling {
            return Status::Checking;
        }
        if self.not_before > now {
            return Status::Cooling(self.not_before.saturating_sub(now));
        }
        Status::Running
    }

    /// The numbers, now.
    #[must_use]
    pub fn progress(&mut self, now: Duration) -> Progress {
        self.tick(now);
        let status = self.status(now);
        let round = self
            .todo
            .front()
            .map(|slot| self.pictures[*slot].round)
            .or_else(|| self.pictures.last().map(|picture| picture.round));
        let (cards_done, cards_total) = round.map_or((0, 0), |round| self.cards[round.index()]);
        let working = matches!(status, Status::Running | Status::Cooling(_));
        let left = self.todo.len() + self.in_flight;
        Progress {
            status,
            round,
            rounds: self.steering.choices.reach.rounds(),
            cards_done,
            cards_total,
            pictures_done: self.pictures_done,
            pictures_total: u32::try_from(self.pictures.len()).unwrap_or(u32::MAX),
            missing: self.missing,
            bytes: self.bytes,
            cached: self.cached,
            budget: self.steering.choices.budget.bytes(),
            elapsed: self.elapsed,
            speed: working.then(|| self.meter.speed(now)).flatten(),
            left: (status == Status::Running)
                .then(|| self.meter.time_for(left, now))
                .flatten(),
        }
    }
}
