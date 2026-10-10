//! Who is here right now, as a number (`GET /lobby/stats`, WG-0).
//!
//! A session row says nothing about presence: a guest's lasts thirty days
//! and an account's twelve hours after its last request, so counting them
//! would call a player who closed the lid last week "online". What does say
//! it is an open lobby socket, so that is what is counted here: each
//! `/lobby/ws` holds a [`Present`] for as long as it is open, and the stats
//! route reads how many distinct accounts hold one. In memory only, never
//! stored, never logged, and never answered as anything but a count.

use parking_lot::Mutex;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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
}

/// One open lobby socket; dropping it is the socket closing.
pub(crate) struct Present {
    /// The map it counts in.
    open: Arc<Mutex<BTreeMap<String, usize>>>,
    /// Whose socket it is.
    account_id: String,
}

impl Presence {
    /// An account opened a lobby socket; it counts until the guard drops.
    pub(crate) fn enter(&self, account_id: &str) -> Present {
        *self.open.lock().entry(account_id.to_owned()).or_default() += 1;
        *self.visits.lock().entry(account_id.to_owned()).or_default() += 1;
        Present {
            open: Arc::clone(&self.open),
            account_id: account_id.to_owned(),
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
        let mut open = self.open.lock();
        if let Some(count) = open.get_mut(&self.account_id) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                open.remove(&self.account_id);
            }
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
}
