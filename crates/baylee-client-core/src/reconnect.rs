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
//! The engine's decision clock does not run for a seat with no socket
//! (`docs/protocol.md`, "The gateway runs no rules"), so nobody is losing a
//! game on time while this waits. That is what permits backing off at all
//! rather than dialling every frame.

/// The retry schedule for a table that has lost its socket.
#[derive(Clone, Debug)]
pub struct Retry {
    /// Seconds left before the next dial.
    left: f32,
    /// The wait this dial was scheduled with, doubling to [`Retry::CAP`].
    step: f32,
    /// Dials made since the link was last up.
    attempts: u32,
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

    /// A schedule for a link that has just gone down.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            left: Self::FIRST,
            step: Self::FIRST,
            attempts: 0,
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

    /// Seconds until the next dial, for a banner that counts down.
    #[must_use]
    pub fn wait(&self) -> f32 {
        self.left.max(0.0)
    }
}

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
        retry.settle();
        assert_eq!(retry.attempts(), 0);
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
