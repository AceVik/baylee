//! When to redial a table whose socket went away.
//!
//! `NetworkHost` has been able to reconnect since it was written — it dials
//! again and queues a `ResumeGame` naming the last sequence it saw — and
//! nothing ever called it. A socket that closed ended the duel with "the
//! connection to the table was lost" on the prompt bar, and no way back to a
//! game that was still sitting there waiting for the seat.
//!
//! What was missing is the policy, and it lives here rather than beside the
//! socket for the reason the lobby's decisions do: a schedule that can only
//! be exercised by disconnecting a real gateway is a schedule that is never
//! tested. This module knows no transport and no renderer. It answers one
//! question — *dial now?* — and the shell performs it.
//!
//! Backing off at all — rather than dialling every frame — is safe because
//! the engine's *decision* clock does not run for a seat with no socket
//! (`docs/protocol.md`, "The gateway runs no rules"): nobody loses a game on
//! time while this waits.
//!
//! That is the wrong clock to reassure anybody with, and this module used it
//! for both jobs. The **reconnect** clock is the one running here, and it is
//! the one with a consequence: after `HouseRules::reconnect_window_secs` the
//! house takes the chair (`Session::stand_in`) and answers for it until the
//! player is back (`SeatAttached` -> `hand_back`). So dialling past that
//! point is right — the chair does come back — and what has to change is the
//! sentence over it, which went on promising that nothing was happening for
//! as long as two minutes. [`Retry::PATIENCE`] is where the wording turns and
//! carries the argument.

/// The retry schedule for a table that has lost its socket.
#[derive(Clone, Debug)]
pub struct Retry {
    /// Seconds left before the next dial.
    left: f32,
    /// The wait this dial was scheduled with, doubling to [`Retry::CAP`].
    step: f32,
    /// Dials made since the link was last up.
    attempts: u32,
    /// Seconds the link has not been up, across dials and the waits between
    /// them.
    ///
    /// Separate from `left` because the two answer different questions.
    /// `left` is about the *schedule*, and it stops while a dial is in
    /// flight; this is about the *player*, who has been gone for that time
    /// whatever the socket was doing. A slow-failing dial is the case that
    /// separates them — thirty seconds inside `Connecting` moves this and
    /// moves nothing else.
    down: f32,
}

impl Default for Retry {
    fn default() -> Self {
        Self::new()
    }
}

impl Retry {
    /// The first wait. Short enough that a socket which closed on a hiccup is
    /// back before the player has finished reading the banner.
    pub const FIRST: f32 = 0.5;

    /// The longest wait between dials. A cap rather than unbounded doubling
    /// because the player is sitting at a table they can see: fifteen seconds
    /// is about as long as "it is still trying" stays believable.
    pub const CAP: f32 = 15.0;

    /// How many dials before the table gives up and says so.
    ///
    /// The loop has to end somewhere. "That game no longer exists" reaches a
    /// client as an ordinary refusal string from the gateway and not as a
    /// state it can match on, so a client that retried forever would sit
    /// redialling a finished game until the player closed the window. Twelve
    /// dials on this schedule is a little over two minutes.
    pub const GIVE_UP: u32 = 12;

    /// How long a drop stays a hiccup, in seconds.
    ///
    /// Not a fact about the table — a fact about the wording. Under it the
    /// player is told the connection dropped and that something is being
    /// done, which is all that is true yet. Over it they are told the rest:
    /// that the house will answer for their seat until they are back.
    ///
    /// **It is deliberately shorter than the shortest reconnect window this
    /// gateway will host** — `MIN_RECONNECT_SECS`, ten seconds, in
    /// `baylee-gateway/src/clock.rs`. That is what makes the second sentence
    /// safe in the future tense: at the moment it first appears no table can
    /// have handed a chair over yet, so it never denies something that has
    /// already happened. The two constants cannot be compared in code —
    /// this crate does not link the gateway and must not — so the assertion
    /// below pins the bound and this names where the other half lives.
    ///
    /// A fixed number is the best a client can do here, and that is the
    /// finding rather than a shortcut. `reconnect_window_secs` is a
    /// per-table value between ten seconds and an hour; it reaches no client
    /// (neither `baylee-view` nor the lobby model carries it); and it could
    /// not be used if it did, because the client is disconnected for exactly
    /// the window it would be counting down.
    pub const PATIENCE: f32 = 8.0;

    /// A schedule for a link that has just gone down.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            left: Self::FIRST,
            step: Self::FIRST,
            attempts: 0,
            down: 0.0,
        }
    }

    /// The link is up: forget everything about having been down.
    pub const fn settle(&mut self) {
        *self = Self::new();
    }

    /// `dt` seconds passed with the link down; whether to dial now.
    ///
    /// The wait doubles on every dial rather than on every failure, because
    /// what a dial reports is not a failure yet — the socket is opened and
    /// then either says `Opened` or closes again, and both take time. Backing
    /// off per dial is what keeps a gateway that is down from being dialled
    /// once a frame by every client that was connected to it.
    pub fn tick(&mut self, dt: f32) -> bool {
        if self.exhausted() {
            return false;
        }
        self.left -= dt;
        if self.left > 0.0 {
            return false;
        }
        self.attempts += 1;
        self.step = (self.step * 2.0).min(Self::CAP);
        self.left = self.step;
        true
    }

    /// `dt` seconds passed with the link not up, whatever it was doing.
    ///
    /// Called on every frame the link is down *or* dialling — both, which is
    /// the whole reason it is not part of [`Retry::tick`]. `tick` must not
    /// run during a dial in flight, or a slow socket would be dialled again
    /// underneath itself; this must, or a slow socket would leave the player
    /// reading the short sentence long after the house had their chair.
    pub fn stayed_down(&mut self, dt: f32) {
        self.down += dt;
    }

    /// Whether the drop is still short enough to mean nothing.
    ///
    /// [`Retry::PATIENCE`] has why there is a threshold and why it is where
    /// it is.
    #[must_use]
    pub fn brief(&self) -> bool {
        self.down < Self::PATIENCE
    }

    /// Whether the schedule has run out and the player has to be told.
    #[must_use]
    pub const fn exhausted(&self) -> bool {
        self.attempts >= Self::GIVE_UP
    }

    /// Dials made since the link was last up.
    #[must_use]
    pub const fn attempts(&self) -> u32 {
        self.attempts
    }

    /// Seconds until the next dial.
    ///
    /// Nothing draws it, and the doc used to say it was "for a banner that
    /// counts down" as though one existed. The bar this client has is
    /// revision-gated on the `Phrase` it carries (`hud::overlay`), so a
    /// number ticking down inside that sentence would rebuild the retained
    /// tree every frame the link was down. It stays because the schedule is
    /// asked about itself in tests, and the doc says what is true.
    #[must_use]
    pub fn wait(&self) -> f32 {
        self.left.max(0.0)
    }
}

/// The bound [`Retry::PATIENCE`] argues for, pinned where it cannot be
/// checked.
///
/// `MIN_RECONNECT_SECS` is ten and lives in the gateway, which this crate
/// does not link. A compile-time assertion against the literal is what is
/// left: it cannot notice the gateway lowering its floor, but it does stop
/// this number being raised past it by somebody who only wanted the banner
/// to wait a little longer.
const _: () = assert!(
    Retry::PATIENCE < 10.0,
    "PATIENCE must stay under the gateway's MIN_RECONNECT_SECS, or the \
     second sentence can appear after a chair has already been handed over"
);

#[cfg(test)]
mod tests {
    use super::*;

    /// The first dial is prompt and every one after it waits longer, up to
    /// the cap. A schedule that started at the cap would make a hiccup look
    /// like an outage; one that never capped would make an outage look like
    /// a hang.
    #[test]
    fn the_first_dial_is_prompt_and_the_rest_back_off() {
        let mut retry = Retry::new();
        assert!(!retry.tick(0.4), "not yet: {}", retry.wait());
        assert!(retry.tick(0.2), "half a second in, dial");
        assert_eq!(retry.attempts(), 1);

        // 1, 2, 4, 8, then the cap holds.
        for expected in [1.0, 2.0, 4.0, 8.0, 15.0, 15.0] {
            assert!(
                (retry.wait() - expected).abs() < 1e-3,
                "waiting {}, expected {expected}",
                retry.wait()
            );
            assert!(!retry.tick(expected - 0.01), "the wait is not over");
            assert!(retry.tick(0.02), "the wait is over");
        }
    }

    /// The wording turns once, at [`Retry::PATIENCE`], and the first seconds
    /// of a drop are not dressed up as an outage. A schedule that reported
    /// the stand-in from frame one would make every hiccup an event.
    #[test]
    fn a_drop_is_a_hiccup_until_it_is_not() {
        let mut retry = Retry::new();
        assert!(retry.brief(), "a link that just went is not an outage");
        retry.stayed_down(Retry::PATIENCE - 0.01);
        assert!(retry.brief(), "still inside the hiccup");
        retry.stayed_down(0.02);
        assert!(!retry.brief(), "past it, and the player is told the rest");
    }

    /// A dial in flight counts against the player even though it counts for
    /// nothing in the schedule.
    ///
    /// This is the whole reason the two are separate calls. `tick` is not
    /// run while a socket is opening — a slow one would be dialled again
    /// underneath itself — so a client that measured the outage by the
    /// schedule alone would sit in `Connecting` for a minute, advance
    /// nothing, and go on saying the connection had just dropped while the
    /// house played the seat.
    #[test]
    fn a_dial_in_flight_still_counts_against_the_player() {
        let mut retry = Retry::new();
        for _ in 0..120 {
            retry.stayed_down(0.5);
        }
        assert_eq!(retry.attempts(), 0, "nothing was dialled");
        assert!(
            (retry.wait() - Retry::FIRST).abs() < 1e-3,
            "the schedule did not move: {}",
            retry.wait()
        );
        assert!(!retry.brief(), "but a minute is not a hiccup");
    }

    /// A link that comes back forgets it was ever down, so the *next* drop
    /// gets the same prompt first dial. Without this, a flaky connection
    /// would take longer to recover each time it dropped, which is exactly
    /// backwards.
    #[test]
    fn a_link_that_comes_back_forgets_it_was_down() {
        let mut retry = Retry::new();
        for _ in 0..4 {
            retry.tick(100.0);
        }
        assert_eq!(retry.attempts(), 4);
        retry.stayed_down(60.0);
        assert!(!retry.brief());
        retry.settle();
        assert_eq!(retry.attempts(), 0);
        assert!(
            retry.brief(),
            "a reconnected table still reads as an outage"
        );
        assert!(!retry.tick(0.4), "the first wait is short again, not zero");
        assert!(retry.tick(0.2));
    }

    /// The schedule stops rather than dialling a dead game forever, and says
    /// so through `exhausted` so the shell can offer the lobby instead of a
    /// banner that spins until the window is closed.
    #[test]
    fn the_schedule_gives_up_rather_than_dialling_forever() {
        let mut retry = Retry::new();
        let mut dials = 0;
        for _ in 0..1000 {
            if retry.tick(100.0) {
                dials += 1;
            }
        }
        assert_eq!(dials, Retry::GIVE_UP, "it stopped where it said it would");
        assert!(retry.exhausted());
        assert!(!retry.tick(100.0), "and stays stopped");
    }

    /// Time is the only thing that moves the schedule: a frame in which no
    /// time passed dials nothing. `tick` is called once per frame, so a
    /// schedule that advanced on calls rather than seconds would dial at a
    /// rate set by the frame rate.
    #[test]
    fn no_dial_happens_without_time_passing() {
        let mut retry = Retry::new();
        for _ in 0..600 {
            assert!(!retry.tick(0.0));
        }
        assert_eq!(retry.attempts(), 0);
    }
}
