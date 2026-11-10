//! Who is here right now, as a number (`GET /lobby/stats`, WG-0).
//!
//! A session row says nothing about presence: a guest's lasts thirty days
//! and an account's twelve hours after its last request, so counting them
//! would call a player who closed the lid last week "online". What does say
//! it is an open lobby socket, so that is what is counted here: each
//! `/lobby/ws` holds a [`Present`] for as long as it is open, and the stats
//! route reads how many distinct accounts hold one. In memory only, never
//! stored, never logged, and never answered as anything but a count.

use crate::{AppState, ErrorBody, Json, StatusCode, auth, err};
use parking_lot::Mutex;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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
    /// Wakes `/lobby/ws` whenever an account's first socket opens or last
    /// socket closes, so `players_online` is pushed immediately over the
    /// lobby WebSocket instead of being polled.
    changed: Option<tokio::sync::broadcast::Sender<()>>,
}

/// One open lobby socket; dropping it is the socket closing.
pub(crate) struct Present {
    /// The map it counts in.
    open: Arc<Mutex<BTreeMap<String, usize>>>,
    /// Whose socket it is.
    account_id: String,
    /// Wakes `/lobby/ws` when the account's last socket closes.
    changed: Option<tokio::sync::broadcast::Sender<()>>,
}

impl Presence {
    /// Builds a presence book that wakes `changed` when `players_online` moves.
    pub(crate) fn with_notifier(changed: tokio::sync::broadcast::Sender<()>) -> Self {
        Self {
            changed: Some(changed),
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
        if first && let Some(changed) = &self.changed {
            let _ = changed.send(());
        }
        Present {
            open: Arc::clone(&self.open),
            account_id: account_id.to_owned(),
            changed: self.changed.clone(),
        }
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
        if last && let Some(changed) = &self.changed {
            let _ = changed.send(());
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
    fn first_and_last_socket_notify_lobby_changed() {
        let (tx, mut rx) = tokio::sync::broadcast::channel(16);
        let presence = Presence::with_notifier(tx);
        let first = presence.enter("a");
        assert!(rx.try_recv().is_ok(), "first socket notifies");
        let second = presence.enter("a");
        assert!(
            rx.try_recv().is_err(),
            "second socket for same account does not notify"
        );
        drop(first);
        assert!(rx.try_recv().is_err(), "still has a socket open");
        drop(second);
        assert!(rx.try_recv().is_ok(), "last socket closing notifies");
    }
}
