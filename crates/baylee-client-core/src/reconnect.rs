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
//!
//! *When* it turns is the table's to say and not this module's. The window
//! is a per-table number, [`Window`] is what a client has been told about it,
//! and the wording turns no later than the chair changes hands at whatever
//! table this is.

use baylee_view::GameStatic;
use std::num::NonZeroU32;

/// How long the table holds a seat whose player is gone, as far as this
/// client knows.
///
/// Three states rather than an `Option<u32>`, because *nobody has told me*
/// and *this table waits forever* are different facts and only one of them
/// is a number that is missing. They happen to want the same banner, which
/// is exactly why flattening them would be a mistake: the agreement would
/// stop looking like a decision and start looking like an accident.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    /// No [`GameStatic`] has arrived, so this client has not been told.
    ///
    /// Reachable: a dial that fails before the payload lands leaves a duel
    /// that knows its seat and not its house rules.
    Unknown,
    /// The chair is held until its player comes back, however long that is.
    ///
    /// `HouseRules::reconnect_window_secs` spells this **zero**, which
    /// `EngineRunner::clock` reads as *no deadline at all* rather than as
    /// *immediately* — `a_table_that_never_gives_up_a_chair_never_takes_one`
    /// in `baylee-engine-server` is the test that pins it.
    Forever,
    /// The house takes the chair after this many seconds.
    ///
    /// Non-zero by construction, so the one number that means the opposite
    /// of what it looks like cannot be stored here at all. [`Window::secs`]
    /// is the door and does the mapping.
    Secs(NonZeroU32),
}

impl Window {
    /// What the once-per-game payload says, if it has arrived.
    ///
    /// The whole mapping lives here so that no caller has to remember which
    /// of `GameStatic`'s two `Option`s means *no limit* — both do, and
    /// `no_limit_is_none` in `baylee-gamehost` is where they are spelt that
    /// way.
    #[must_use]
    pub fn of(statics: Option<&GameStatic>) -> Self {
        statics.map_or(Self::Unknown, |statics| {
            statics.reconnect_secs.map_or(Self::Forever, Self::secs)
        })
    }

    /// A window of `secs` seconds, reading zero the way the house rules
    /// write it: a table that waits forever.
    #[must_use]
    pub fn secs(secs: u32) -> Self {
        NonZeroU32::new(secs).map_or(Self::Forever, Self::Secs)
    }
}

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

    /// The longest a drop is dressed up as a hiccup, in seconds.
    ///
    /// Not a fact about the table — a fact about the wording. Under it the
    /// player is told the connection dropped and that something is being
    /// done, which is all that is true yet. Over it they are told the rest:
    /// that the house will answer for their seat until they are back.
    ///
    /// **A cap, and no longer a claim.** It used to be justified by being
    /// under `MIN_RECONNECT_SECS` — the gateway's floor of ten on a reconnect
    /// window — so that the second sentence could never appear after a chair
    /// had already changed hands. That argument was about a constant in a
    /// process this crate does not link, and it only held where the gateway
    /// was the thing that made the table: the engine accepts any window at
    /// all, so a harness could seat one this number outran. The old doc named
    /// that case and said it was worth saying rather than guarding, and a
    /// compile-time assertion pinned the literal. Both are gone.
    /// [`Retry::brief`] takes the window the table actually plays at and
    /// turns the wording at whichever comes first, so no value of this
    /// constant can outrun any window and there is nothing left to pin.
    ///
    /// What it is now is eight seconds of reading room on a table whose
    /// window is long: at a one-hour window the second sentence would
    /// otherwise wait an hour, and a player watching a bar that has said
    /// nothing new for a minute has been told less than one that turned.
    /// Nothing measured chose eight; it is the number that was already here,
    /// kept because a cap has to be some number and this one has been on
    /// screen.
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

    /// Whether the drop is still short enough to mean nothing **at this
    /// table**.
    ///
    /// The wording turns at whichever comes first, the cap or the window, so
    /// the sentence that says the house will answer for the seat is only ever
    /// shown while that is still ahead. At a table that hands the chair over
    /// after five seconds it turns at five; at one that never does, or at one
    /// this client has not been told about, it does not turn at all and the
    /// player goes on being told the true thing — that the connection
    /// dropped and it is being dialled.
    ///
    /// [`Retry::PATIENCE`] has why there is a cap; [`Window`] has why not
    /// knowing and waiting forever are separate and land in the same arm.
    #[must_use]
    pub fn brief(&self, window: Window) -> bool {
        match window {
            Window::Unknown | Window::Forever => true,
            // In `f64` rather than casting the window to `f32`: every `u32`
            // a window can hold is exact there, and the comparison is the
            // one place a rounded second would move a sentence.
            Window::Secs(secs) => {
                f64::from(self.down) < f64::from(Self::PATIENCE).min(f64::from(secs.get()))
            }
        }
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
        // An ordinary table: its window is far longer than the cap, so the
        // cap is what turns the wording and this test is about the cap.
        let table = Window::secs(60);
        let mut retry = Retry::new();
        assert!(retry.brief(table), "a link that just went is not an outage");
        retry.stayed_down(Retry::PATIENCE - 0.01);
        assert!(retry.brief(table), "still inside the hiccup");
        retry.stayed_down(0.02);
        assert!(
            !retry.brief(table),
            "past it, and the player is told the rest"
        );
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
        assert!(
            !retry.brief(Window::secs(60)),
            "but a minute is not a hiccup"
        );
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
        assert!(!retry.brief(Window::secs(60)));
        retry.settle();
        assert_eq!(retry.attempts(), 0);
        assert!(
            retry.brief(Window::secs(60)),
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

    /// The wording turns while the handover is still ahead, at every table
    /// this client can be told about.
    ///
    /// This is what replaced the compile-time assertion pinning `PATIENCE`
    /// under the gateway's floor of ten. That bound only spoke for tables the
    /// gateway made; the range here deliberately straddles the cap, because a
    /// window **below** it is exactly the table the old bound could not see
    /// and the engine will happily seat.
    ///
    /// The turn is written out beside the window rather than computed, so
    /// this does not check the implementation's `min` against a second copy
    /// of itself.
    #[test]
    fn the_wording_turns_no_later_than_the_chair_changes_hands() {
        for (secs, turn) in [
            (1_u32, 1.0_f32),
            (5, 5.0),
            (8, 8.0),
            (9, 8.0),
            (10, 8.0),
            (60, 8.0),
            (3600, 8.0),
        ] {
            let table = Window::secs(secs);
            let mut retry = Retry::new();
            retry.stayed_down(turn - 0.01);
            assert!(
                retry.brief(table),
                "a {secs}s table turned the wording before {turn}s"
            );
            retry.stayed_down(0.02);
            assert!(
                !retry.brief(table),
                "a {secs}s table had not turned the wording by {turn}s"
            );
        }
    }

    /// A table that never takes a chair is never said to be about to.
    ///
    /// The second sentence promises a handover; where there is no handover it
    /// is not early, it is false, and it stays false for as long as the
    /// player is away. `LinkLost` — the connection dropped and it is being
    /// dialled — is true the whole time instead.
    #[test]
    fn a_table_that_waits_forever_never_reaches_the_second_sentence() {
        let mut retry = Retry::new();
        for _ in 0..240 {
            retry.stayed_down(0.5);
            assert!(retry.brief(Window::Forever));
        }
    }

    /// A client that was never told promises nothing either.
    ///
    /// The one state that is not about the table at all. It lands in the same
    /// arm as [`Window::Forever`] and for a different reason: there the
    /// handover is known not to be coming, here it is unknown, and a sentence
    /// that asserts one is a fabrication in both cases.
    #[test]
    fn a_table_this_client_was_never_told_about_promises_nothing() {
        let mut retry = Retry::new();
        retry.stayed_down(3600.0);
        assert!(retry.brief(Window::Unknown));
    }

    /// Zero is the table that waits forever, not the one that takes the chair
    /// at once — the house rules' own spelling, and the opposite of what the
    /// number looks like.
    ///
    /// Pinned here because reading it the other way is a one-character
    /// mistake that produces a banner which is wrong from the first frame,
    /// and because `Window::Secs` is the type that makes it unrepresentable.
    #[test]
    fn a_zero_window_is_read_as_forever_and_not_as_at_once() {
        assert_eq!(Window::secs(0), Window::Forever);
        assert_eq!(Window::of(None), Window::Unknown);

        let mut statics = crate::test_support::statics(0);
        statics.reconnect_secs = None;
        assert_eq!(Window::of(Some(&statics)), Window::Forever);
        statics.reconnect_secs = Some(30);
        assert_eq!(Window::of(Some(&statics)), Window::secs(30));
    }
}
