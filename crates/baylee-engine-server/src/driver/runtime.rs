use crate::{Clock, EngineRunner, Envelope, LoopDetection, Session, SessionRegistry};

impl<D: LoopDetection, R: SessionRegistry, T: crate::recording::Transport> EngineRunner<D, R, T> {
    /// Sets the maximum number of house steps answered per call before
    /// yielding intermediate state deltas to the event loop.
    #[must_use]
    pub const fn with_pump_budget(mut self, budget: usize) -> Self {
        self.pump_budget = if budget == 0 { 1 } else { budget };
        self
    }

    /// Has the house search at the game this runner builds, within each
    /// level's own bound and `host`'s (`foresight::Settings::host`); `None`,
    /// the default, plays every level as its heuristics do. An attached
    /// engine reads it from its environment (`foresight::Settings`).
    #[must_use]
    pub fn with_foresight(mut self, host: Option<baylee_gamehost::HostForesight>) -> Self {
        self.foresight = host;
        self
    }

    /// The most answers the house gives alone once nobody is left at the
    /// table, in place of [`crate::REMAINDER_ANSWERS`]: for a test.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn with_remainder_answers(mut self, answers: u64) -> Self {
        self.remainder_answers = answers;
        self
    }

    /// Updates the house step budget per slice.
    pub fn set_pump_budget(&mut self, budget: usize) {
        self.pump_budget = if budget == 0 { 1 } else { budget };
    }

    /// Whether the open table still owes house AI steps that should be
    /// advanced on the next event-loop tick via [`Self::pump_slice`].
    #[must_use]
    pub fn needs_pump(&self) -> bool {
        !self.ended
            && self.curtain.is_none()
            && self.session.as_ref().is_some_and(Session::needs_pump)
    }

    /// Advances the house AI by one slice (`self.pump_budget` steps) and
    /// returns the routed envelopes (intermediate [`v1::StateDelta`] frames
    /// while `needs_pump()` remains true, or the final view and
    /// [`v1::ChoiceRequest`] once a socket seat is reached).
    pub fn pump_slice(&mut self, remaining: &[(Clock, u32)]) -> Vec<Envelope> {
        if !self.needs_pump() {
            return Vec::new();
        }
        self.set_readings(remaining);
        self.absorb_clock();
        let budget = self.pump_budget;
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        let routed = session.pump_slice(budget);
        let mut out = self.route(&routed);
        out.extend(self.ending());
        out
    }

    pub(crate) fn set_readings(&mut self, remaining: &[(Clock, u32)]) {
        if remaining.is_empty() {
            self.readings.clear();
        } else {
            self.readings.clear();
            self.readings.extend_from_slice(remaining);
        }
    }

    /// Tells the game the wall time, in milliseconds since the Unix epoch,
    /// which its log stamps each line with (#300). The caller tells it before
    /// every call that can move the game: the runner reads no clock, for the
    /// reason [`Self::timeout`] gives.
    pub fn tell_time(&mut self, unix_ms: u64) {
        self.now = unix_ms;
        if let Some(session) = self.session.as_mut() {
            session.tell_time(unix_ms);
        }
    }

    /// Whether the game has been built.
    #[must_use]
    pub fn ready(&self) -> bool {
        self.session.is_some()
    }

    /// Whether the table has been built and not yet opened (#256): what
    /// the caller arms [`CURTAIN_SECS`] against, and what it disarms on.
    ///
    /// Nothing is decided while this holds. No seat is asked a question, an
    /// action that arrives is dropped, the house plays no AI chair, and no
    /// clock runs.
    #[must_use]
    pub const fn curtain_pending(&self) -> bool {
        self.curtain.is_some()
    }

    /// Whether the game is over and the process may exit.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.ended
    }

    /// The game underneath, for tests and for a harness that wants to look.
    #[must_use]
    pub fn session(&self) -> Option<&Session<D, R>> {
        self.session.as_ref()
    }
}
