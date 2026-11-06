//! Every session as this gateway holds it in memory: one state that all of
//! the session's sockets watch (`docs/central-login.md` §"Sessions").
//!
//! The check before a frame goes to an engine is a read of that state, with
//! no network and no SQL. Ending a session flips it once, and every socket
//! that watches it closes at once, in both directions.
//!
//! A session is known here by a stable [`Key`], never by the bearer it is
//! currently reached with: a Central session's bearer changes every minute or
//! two, and a chair bound to one must keep its session across that.
#![forbid(unsafe_code)]

use parking_lot::Mutex;
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    },
};
use tokio::sync::watch;

/// How many sessions the gateway holds at once. A sign-in past it is refused
/// rather than letting memory grow with whoever keeps signing in.
const MAX_SESSIONS: usize = 262_144;

/// What identifies a session here, for as long as it lives.
pub(crate) type Key = Vec<u8>;

/// The clock every check here reads, in Unix seconds.
pub(crate) fn now() -> i64 {
    i64::try_from(crate::auth::now_secs()).unwrap_or(i64::MAX)
}

/// Why a session ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Ended {
    /// Its player signed out, here or at Central.
    SignedOut,
    /// Central ended it: a ban, a newer sign-in, a removed admin.
    Revoked,
    /// This gateway banned the account.
    Banned,
    /// The account is gone.
    Deleted,
    /// It ran out of time.
    Expired,
}

/// One session's state. Sockets hold this and read it; nothing here touches
/// the network or the database.
pub(crate) struct Live {
    key: Key,
    account: String,
    expires_at: AtomicI64,
    end: watch::Sender<Option<Ended>>,
}

impl Live {
    fn new(key: Key, account: String, expires_at: i64) -> Arc<Self> {
        Arc::new(Self {
            key,
            account,
            expires_at: AtomicI64::new(expires_at),
            end: watch::Sender::new(None),
        })
    }

    /// The stable key a chair is bound to.
    pub(crate) fn key(&self) -> &[u8] {
        &self.key
    }

    /// Whose session this is.
    pub(crate) fn account(&self) -> &str {
        &self.account
    }

    /// When it runs out, in Unix seconds.
    pub(crate) fn expires_at(&self) -> i64 {
        self.expires_at.load(Ordering::Acquire)
    }

    /// Whether it stands at `now`: the check before every frame. Time is read
    /// here as well as by the sweep, so a late sweep lets nothing through.
    pub(crate) fn stands(&self, now: i64) -> bool {
        self.end.borrow().is_none() && now < self.expires_at()
    }

    /// Ends it, once: the first reason is the one that stays. Every socket
    /// watching it wakes.
    pub(crate) fn end(&self, why: Ended) {
        self.end.send_if_modified(|state| {
            if state.is_some() {
                return false;
            }
            *state = Some(why);
            true
        });
    }

    /// Resolves when the session ends, at once if it already has. Safe in a
    /// `select!`: dropping it loses nothing, since the state stays set.
    pub(crate) async fn ended(&self) -> Ended {
        let mut watcher = self.end.subscribe();
        match watcher.wait_for(Option::is_some).await {
            Ok(state) => (*state).unwrap_or(Ended::Expired),
            // The sender lives in `self`, so it outlives this borrow.
            Err(_) => Ended::Expired,
        }
    }

    /// Moves the end later, never earlier: a fresh gateway token may extend a
    /// session and an old one presented late may not shorten it.
    fn extend(&self, expires_at: i64) {
        self.expires_at.fetch_max(expires_at, Ordering::AcqRel);
    }
}

#[derive(Default)]
struct Index {
    by_key: HashMap<Key, Arc<Live>>,
    by_account: HashMap<String, HashSet<Key>>,
}

impl Index {
    fn forget(&mut self, key: &[u8]) -> Option<Arc<Live>> {
        let live = self.by_key.remove(key)?;
        if let Some(keys) = self.by_account.get_mut(live.account())
            && keys.remove(key)
            && keys.is_empty()
        {
            self.by_account.remove(live.account());
        }
        Some(live)
    }
}

/// Every live session. One lock, held only for map work: nothing is awaited
/// under it and it is not taken per frame, since a socket holds its [`Live`].
#[derive(Default)]
pub(crate) struct Book {
    index: Mutex<Index>,
}

/// The book is full; the sign-in is refused.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Full;

impl Book {
    /// The session at `key`, made if it is not here, its end moved to
    /// `expires_at` if that is later. A key whose session ended is not
    /// revived: the caller gets a new state, and sockets of the old one stay
    /// closed.
    ///
    /// # Errors
    /// [`Full`] when a new session would pass [`MAX_SESSIONS`].
    pub(crate) fn open(
        &self,
        key: Key,
        account: &str,
        expires_at: i64,
        now: i64,
    ) -> Result<Arc<Live>, Full> {
        let mut index = self.index.lock();
        if let Some(live) = index.by_key.get(&key).cloned() {
            if live.stands(now) && live.account() == account {
                live.extend(expires_at);
                return Ok(live);
            }
            live.end(Ended::Expired);
            index.forget(&key);
        }
        if index.by_key.len() >= MAX_SESSIONS {
            return Err(Full);
        }
        let live = Live::new(key.clone(), account.to_owned(), expires_at);
        index
            .by_account
            .entry(account.to_owned())
            .or_default()
            .insert(key.clone());
        index.by_key.insert(key, Arc::clone(&live));
        Ok(live)
    }

    /// The session at `key` while it stands.
    pub(crate) fn get(&self, key: &[u8], now: i64) -> Option<Arc<Live>> {
        self.index
            .lock()
            .by_key
            .get(key)
            .filter(|live| live.stands(now))
            .cloned()
    }

    /// Ends the session at `key`, whether or not it is here.
    pub(crate) fn end(&self, key: &[u8], why: Ended) {
        if let Some(live) = self.index.lock().forget(key) {
            live.end(why);
        }
    }

    /// Ends every session of `account`: a ban, a deletion.
    pub(crate) fn end_account(&self, account: &str, why: Ended) {
        let mut index = self.index.lock();
        let keys = index.by_account.remove(account).unwrap_or_default();
        for key in keys {
            if let Some(live) = index.by_key.remove(&key) {
                live.end(why);
            }
        }
    }

    /// Ends every session `which` picks out, for a gateway that may have
    /// missed a revocation it can no longer learn.
    pub(crate) fn end_all(&self, which: impl Fn(&Live) -> bool, why: Ended) {
        let mut index = self.index.lock();
        let keys: Vec<Key> = index
            .by_key
            .iter()
            .filter(|(_, live)| which(live))
            .map(|(key, _)| key.clone())
            .collect();
        for key in keys {
            if let Some(live) = index.forget(&key) {
                live.end(why);
            }
        }
    }

    /// Ends what ran out by `now` and forgets it, so that an idle socket
    /// closes on time and the book stays the size of what is live.
    pub(crate) fn sweep(&self, now: i64) {
        self.end_all(|live| now >= live.expires_at(), Ended::Expired);
    }

    /// How many sessions are held.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.index.lock().by_key.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const NOW: i64 = 1_000;
    const LATER: i64 = 2_000;

    fn key(n: u8) -> Key {
        vec![n; 32]
    }

    #[test]
    fn a_session_stands_until_it_runs_out_even_if_no_sweep_came() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW).unwrap();
        assert!(live.stands(NOW));
        assert!(live.stands(LATER - 1));
        assert!(!live.stands(LATER), "the end itself is already too late");
        assert!(book.get(&key(1), LATER).is_none());
    }

    #[test]
    fn ending_flips_every_holder_and_keeps_the_first_reason() {
        let book = Book::default();
        let socket = book.open(key(1), "a", LATER, NOW).unwrap();
        let other = book.open(key(1), "a", LATER, NOW).unwrap();
        assert!(Arc::ptr_eq(&socket, &other), "one state per session");
        book.end(&key(1), Ended::Revoked);
        assert!(!socket.stands(NOW));
        assert!(!other.stands(NOW));
        socket.end(Ended::SignedOut);
        assert_eq!(*socket.end.borrow(), Some(Ended::Revoked));
        assert_eq!(book.len(), 0);
    }

    #[tokio::test]
    async fn a_watcher_wakes_at_the_end_and_one_that_comes_late_returns_at_once() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW).unwrap();
        let watching = Arc::clone(&live);
        let watcher = tokio::spawn(async move { watching.ended().await });
        tokio::task::yield_now().await;
        book.end(&key(1), Ended::Banned);
        let woke = tokio::time::timeout(Duration::from_secs(5), watcher).await;
        assert_eq!(woke.unwrap().unwrap(), Ended::Banned);
        let late = tokio::time::timeout(Duration::from_secs(5), live.ended()).await;
        assert_eq!(late.unwrap(), Ended::Banned);
    }

    #[test]
    fn an_ended_key_is_never_revived_for_its_old_sockets() {
        let book = Book::default();
        let old = book.open(key(1), "a", LATER, NOW).unwrap();
        book.end(&key(1), Ended::Revoked);
        let new = book.open(key(1), "a", LATER, NOW).unwrap();
        assert!(!Arc::ptr_eq(&old, &new));
        assert!(!old.stands(NOW), "the old socket stays closed");
        assert!(new.stands(NOW));
    }

    #[test]
    fn a_key_never_passes_to_another_account() {
        let book = Book::default();
        let first = book.open(key(1), "a", LATER, NOW).unwrap();
        let second = book.open(key(1), "b", LATER, NOW).unwrap();
        assert!(!first.stands(NOW));
        assert_eq!(second.account(), "b");
        book.end_account("a", Ended::Banned);
        assert!(second.stands(NOW), "the old account's index is clean");
    }

    #[test]
    fn the_end_moves_later_and_never_earlier() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW).unwrap();
        book.open(key(1), "a", LATER + 100, NOW).unwrap();
        assert_eq!(live.expires_at(), LATER + 100);
        book.open(key(1), "a", LATER - 100, NOW).unwrap();
        assert_eq!(live.expires_at(), LATER + 100);
    }

    #[test]
    fn a_ban_ends_every_session_of_that_account_and_no_other() {
        let book = Book::default();
        let first = book.open(key(1), "a", LATER, NOW).unwrap();
        let second = book.open(key(2), "a", LATER, NOW).unwrap();
        let other = book.open(key(3), "b", LATER, NOW).unwrap();
        book.end_account("a", Ended::Banned);
        assert!(!first.stands(NOW) && !second.stands(NOW));
        assert!(other.stands(NOW));
        assert_eq!(book.len(), 1);
    }

    #[test]
    fn the_sweep_ends_what_ran_out_and_leaves_the_rest() {
        let book = Book::default();
        let gone = book.open(key(1), "a", NOW + 1, NOW).unwrap();
        let kept = book.open(key(2), "a", LATER, NOW).unwrap();
        book.sweep(NOW + 1);
        assert_eq!(*gone.end.borrow(), Some(Ended::Expired));
        assert!(kept.stands(NOW + 1));
        assert_eq!(book.len(), 1);
    }

    #[test]
    fn a_full_book_refuses_a_new_session_and_still_finds_the_ones_it_holds() {
        let book = Book::default();
        let live = book.open(key(0), "a", LATER, NOW).unwrap();
        {
            let mut index = book.index.lock();
            for n in 1..MAX_SESSIONS {
                let filler = n.to_be_bytes().to_vec();
                let held = Live::new(filler.clone(), "b".into(), LATER);
                index.by_key.insert(filler, held);
            }
        }
        assert_eq!(book.open(key(9), "c", LATER, NOW).err(), Some(Full));
        let same = book.open(key(0), "a", LATER, NOW).unwrap();
        assert!(Arc::ptr_eq(&live, &same));
    }
}
