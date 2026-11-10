//! Who is here right now, as a number (`GET /lobby/stats`, WG-0).
//!
//! A session row says nothing about presence: a guest's lasts thirty days
//! and an account's twelve hours after its last request, so counting them
//! would call a player who closed the lid last week "online". What does say
//! it is an open lobby socket, so that is what is counted here: each
//! `/lobby/ws` holds a [`Present`] for as long as it is open, and the stats
//! route reads how many distinct accounts hold one. In memory only, never
//! stored, never logged, and never answered as anything but a count.

use crate::{AppState, ErrorBody, Json, Shared, StatusCode, auth, err};
use parking_lot::Mutex;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How often an account's first socket opening or last closing may wake
/// `/lobby/ws`.
///
/// Each wake has every lobby socket render its page again, the online count
/// with it, and any account can open and close sockets as fast as it likes.
/// So the first coming or going after a quiet round is told at once, and
/// whatever comes or goes inside the round once, at its end.
const WAKE_AT_MOST: Duration = Duration::from_secs(1);

/// How many sockets one account (or one chair's seat token) may hold open.
///
/// A client holds a lobby feed and a social socket per device and a watch or
/// seat socket beside them, so this leaves room for several devices and for
/// a flapping connection whose dead sockets have not timed out yet. Every
/// socket is read and asked after its session every five seconds for as long
/// as it stays open, which is why it has a bound at all.
pub(crate) const SOCKETS_PER_HOLDER: usize = 32;

/// A place for one more of `holder`'s sockets, held for as long as the
/// socket is open, or `429` while it holds all of its own.
pub(crate) fn socket_place(
    state: &AppState,
    holder: &str,
) -> Result<auth::Held, (StatusCode, Json<ErrorBody>)> {
    state
        .sockets
        .take(holder)
        .ok_or_else(|| err(StatusCode::TOO_MANY_REQUESTS, "too many sockets"))
}

/// The accounts with a lobby socket open, and how many each has.
#[derive(Default)]
pub(crate) struct Presence {
    /// Open sockets per account. An account with two windows open is one
    /// player, so the count is kept per account and read as keys.
    open: Arc<Mutex<BTreeMap<String, usize>>>,
    /// How many sockets each account has ever opened here. A departure
    /// armed when the last one closed reads it again when it falls due: a
    /// different number is a player who came back in between, even if they
    /// have left again since and the later departure is the one to act.
    visits: Mutex<BTreeMap<String, u64>>,
    /// Wakes `/lobby/ws` when an account's first socket opens or its last
    /// closes, so `players_online` is pushed rather than polled; `None` in a
    /// book nobody listens to.
    wake: Option<Arc<Wake>>,
}

/// One open lobby socket; dropping it is the socket closing.
pub(crate) struct Present {
    /// The map it counts in.
    open: Arc<Mutex<BTreeMap<String, usize>>>,
    /// Whose socket it is.
    account_id: String,
    /// Wakes `/lobby/ws` when the account's last socket closes.
    wake: Option<Arc<Wake>>,
}

/// The lobby's wake for comings and goings, at most once a round
/// ([`WAKE_AT_MOST`]).
struct Wake {
    /// `lobby_changed`.
    changed: tokio::sync::broadcast::Sender<()>,
    /// How long a round lasts.
    round: Duration,
    /// When the lobby was last woken for a coming or going, and whether
    /// somebody came or went since and waits for the round's end.
    last: Mutex<(Option<Instant>, bool)>,
}

impl Wake {
    /// Somebody came or went at `now`: the lobby hears it at once unless it
    /// already heard of one this round, and then at the round's end.
    fn came_or_went(&self, now: Instant) {
        let at_once = {
            let mut last = self.last.lock();
            let at_once = !self.in_round(last.0, now);
            if at_once {
                last.0 = Some(now);
            }
            // A wake now says what was held as well, for each socket renders
            // the lobby as it is when it wakes.
            last.1 = !at_once;
            at_once
        };
        if at_once {
            let _ = self.changed.send(());
        }
    }

    /// Tells what a round held back, once the round is over at `now`.
    fn flush(&self, now: Instant) {
        let due = {
            let mut last = self.last.lock();
            let due = last.1 && !self.in_round(last.0, now);
            if due {
                *last = (Some(now), false);
            }
            due
        };
        if due {
            let _ = self.changed.send(());
        }
    }

    /// Whether `now` falls inside the round a wake at `woken` began.
    fn in_round(&self, woken: Option<Instant>, now: Instant) -> bool {
        woken.is_some_and(|woken| now.saturating_duration_since(woken) < self.round)
    }
}

/// Tells `/lobby/ws` the comings and goings a round held back, a few times
/// a round.
pub(crate) fn spawn(state: Shared) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(WAKE_AT_MOST / 4);
        loop {
            tick.tick().await;
            state.presence.flush(Instant::now());
        }
    });
}

impl Presence {
    /// Builds a presence book that wakes `changed` when `players_online`
    /// moves, at most once a round ([`WAKE_AT_MOST`], [`spawn`]).
    pub(crate) fn with_notifier(changed: tokio::sync::broadcast::Sender<()>) -> Self {
        Self::waking(changed, WAKE_AT_MOST)
    }

    /// A presence book that wakes `changed` at most once every `round`.
    fn waking(changed: tokio::sync::broadcast::Sender<()>, round: Duration) -> Self {
        Self {
            wake: Some(Arc::new(Wake {
                changed,
                round,
                last: Mutex::new((None, false)),
            })),
            ..Self::default()
        }
    }

    /// An account opened a lobby socket; it counts until the guard drops.
    pub(crate) fn enter(&self, account_id: &str) -> Present {
        let first = {
            let mut open = self.open.lock();
            let count = open.entry(account_id.to_owned()).or_default();
            *count += 1;
            *count == 1
        };
        *self.visits.lock().entry(account_id.to_owned()).or_default() += 1;
        if first && let Some(wake) = &self.wake {
            wake.came_or_went(Instant::now());
        }
        Present {
            open: Arc::clone(&self.open),
            account_id: account_id.to_owned(),
            wake: self.wake.clone(),
        }
    }

    /// Tells the lobby what the last round held back, if it is over at `now`.
    pub(crate) fn flush(&self, now: Instant) {
        if let Some(wake) = &self.wake {
            wake.flush(now);
        }
    }

    /// How many distinct accounts hold a socket open or are among `also`,
    /// without copying either.
    pub(crate) fn count_with(&self, also: &BTreeSet<String>) -> usize {
        let open = self.open.lock();
        open.len() + also.iter().filter(|id| !open.contains_key(*id)).count()
    }

    /// Whether `account_id` holds a socket open now.
    pub(crate) fn here(&self, account_id: &str) -> bool {
        self.open.lock().contains_key(account_id)
    }

    /// How many sockets `account_id` has opened since this process started.
    pub(crate) fn visits(&self, account_id: &str) -> u64 {
        self.visits.lock().get(account_id).copied().unwrap_or(0)
    }

    /// The accounts with at least one socket open.
    pub(crate) fn accounts(&self) -> BTreeSet<String> {
        self.open.lock().keys().cloned().collect()
    }
}

impl Drop for Present {
    fn drop(&mut self) {
        let last = {
            let mut open = self.open.lock();
            if let Some(count) = open.get_mut(&self.account_id) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    open.remove(&self.account_id);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };
        if last && let Some(wake) = &self.wake {
            wake.came_or_went(Instant::now());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_account_counts_once_however_many_sockets_it_holds_and_leaves_with_the_last() {
        let presence = Presence::default();
        let first = presence.enter("a");
        let second = presence.enter("a");
        let other = presence.enter("b");
        assert_eq!(presence.accounts().len(), 2);
        drop(first);
        assert!(presence.accounts().contains("a"), "left with a socket open");
        drop(second);
        assert_eq!(
            presence.accounts().into_iter().collect::<Vec<_>>(),
            ["b"],
            "stayed after its last socket closed"
        );
        drop(other);
        assert!(presence.accounts().is_empty());
        assert!(!presence.here("a"));
        assert_eq!(presence.visits("a"), 2, "a visit is counted once it began");
        let _back = presence.enter("a");
        assert!(presence.here("a"));
        assert_eq!(presence.visits("a"), 3);
    }

    #[test]
    fn first_and_last_sockets_wake_the_lobby_at_most_once_a_round() {
        let (tx, mut rx) = tokio::sync::broadcast::channel(16);
        // A round no test outlasts, so what is held stays held until told.
        let round = Duration::from_secs(3600);
        let presence = Presence::waking(tx, round);
        let first = presence.enter("a");
        assert!(rx.try_recv().is_ok(), "the first socket wakes the lobby");
        let second = presence.enter("a");
        drop(first);
        assert!(
            rx.try_recv().is_err(),
            "a second socket of the same account is no news"
        );
        // Opening and closing sockets as fast as one can wakes nobody more.
        for _ in 0..1000 {
            drop(presence.enter("b"));
        }
        drop(second);
        assert!(
            rx.try_recv().is_err(),
            "comings and goings inside the round wait for its end"
        );
        presence.flush(Instant::now());
        assert!(rx.try_recv().is_err(), "nor before it");
        presence.flush(Instant::now() + round);
        assert!(rx.try_recv().is_ok(), "they are told at the round's end");
        assert!(rx.try_recv().is_err(), "once");
        presence.flush(Instant::now() + 3 * round);
        assert!(rx.try_recv().is_err(), "and then nothing is left to tell");
    }

    #[test]
    fn after_a_quiet_round_the_next_coming_is_told_at_once_and_says_what_was_held() {
        let (tx, mut rx) = tokio::sync::broadcast::channel(16);
        let round = Duration::from_secs(1);
        let presence = Presence::waking(tx, round);
        let wake = presence.wake.as_deref().expect("a waking book");
        let start = Instant::now();
        wake.came_or_went(start);
        assert!(rx.try_recv().is_ok(), "told at once");
        wake.came_or_went(start + round / 2);
        assert!(rx.try_recv().is_err(), "held inside the round");
        wake.came_or_went(start + round * 2);
        assert!(rx.try_recv().is_ok(), "a quiet round later, at once again");
        wake.flush(start + round * 4);
        assert!(
            rx.try_recv().is_err(),
            "what was held went with the wake after it, not twice"
        );
    }

    #[test]
    fn a_count_with_others_counts_each_account_once() {
        let presence = Presence::default();
        let _a = presence.enter("a");
        let _b = presence.enter("b");
        let bridging: BTreeSet<String> = ["b", "c"].map(str::to_owned).into();
        assert_eq!(presence.count_with(&bridging), 3);
        assert_eq!(presence.count_with(&BTreeSet::new()), 2);
    }
}
