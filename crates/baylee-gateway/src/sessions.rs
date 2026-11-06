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

/// The key of an account's own state: what a seat with no session stands on.
/// Longer than any digest, so it cannot collide with one.
pub(crate) fn account_key(account: &str) -> Key {
    format!("account:{account}:with-no-session").into_bytes()
}

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
    /// A guest's or a local account's: the database holds its lifetime, and
    /// `central::spawn_local_check` asks it, away from any frame.
    local: bool,
    /// What Central's token said, for a Central session.
    signed: Option<Signed>,
}

/// What a gateway token said of its session, so that a request needs no
/// word from Central. A later token of the session may say other terms.
pub(crate) struct Signed {
    /// The account at Central.
    pub(crate) central: [u8; 16],
    /// An `admin` session, which works on the admin listener and nowhere else.
    pub(crate) admin: bool,
    /// The newest terms of this gateway the account accepted.
    terms: Mutex<Option<central_crypto::jwt::TermsClaim>>,
}

impl Signed {
    pub(crate) fn new(
        central: [u8; 16],
        admin: bool,
        terms: Option<central_crypto::jwt::TermsClaim>,
    ) -> Self {
        Self {
            central,
            admin,
            terms: Mutex::new(terms),
        }
    }

    /// Whether the account accepted exactly these terms.
    pub(crate) fn accepted(&self, version: &str, digest: &str) -> bool {
        self.terms
            .lock()
            .as_ref()
            .is_some_and(|terms| terms.version == version && terms.digest == digest)
    }

    /// What a fresh token of this session says.
    pub(crate) fn set_terms(&self, terms: Option<central_crypto::jwt::TermsClaim>) {
        *self.terms.lock() = terms;
    }
}

impl Live {
    fn new(
        key: Key,
        account: String,
        expires_at: i64,
        local: bool,
        signed: Option<Signed>,
    ) -> Arc<Self> {
        Arc::new(Self {
            key,
            account,
            expires_at: AtomicI64::new(expires_at),
            end: watch::Sender::new(None),
            local,
            signed,
        })
    }

    /// What Central's token said, for a Central session.
    pub(crate) const fn signed(&self) -> Option<&Signed> {
        self.signed.as_ref()
    }

    /// Whether Central signed this session in. A session ended for a link
    /// that went is told apart from a guest's by this.
    pub(crate) const fn is_central(&self) -> bool {
        self.signed.is_some()
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

    /// The stable reason a socket of this session closes with
    /// (`docs/protocol.md` §"Refusals").
    pub(crate) fn refusal_code(&self) -> &'static str {
        match *self.end.borrow() {
            Some(Ended::Banned) => baylee_protocol::refusal::BANNED,
            _ => baylee_protocol::refusal::SESSION_ENDED,
        }
    }

    /// The close frame of a socket of this session: its refusal once the
    /// session ended, none while it stands.
    pub(crate) fn close_frame(&self) -> Option<axum::extract::ws::CloseFrame> {
        (!self.stands(now())).then(|| axum::extract::ws::CloseFrame {
            code: baylee_protocol::refusal::REFUSAL_CLOSE,
            reason: self.refusal_code().into(),
        })
    }

    /// Moves the end later, never earlier: a fresh gateway token may extend a
    /// session and an old one presented late may not shorten it.
    pub(crate) fn extend(&self, expires_at: i64) {
        self.expires_at.fetch_max(expires_at, Ordering::AcqRel);
    }
}

/// A bearer a session is reached with, known here only by its digest.
struct Bearer {
    key: Key,
    expires_at: i64,
    /// A later bearer replaced this one: it still serves what was in flight
    /// until it runs out, and renews nothing.
    replaced: bool,
}

#[derive(Default)]
struct Index {
    by_key: HashMap<Key, Arc<Live>>,
    by_account: HashMap<String, HashSet<Key>>,
    /// Looked up by the SHA-256 of a 256-bit random token, so a timing
    /// difference in the map says nothing about the token.
    bearers: HashMap<Vec<u8>, Bearer>,
    /// Each session's bearers, so that ending many sessions at once stays
    /// linear under this lock. A few at most: the current one and any still
    /// in their overlap.
    bearers_of: HashMap<Key, Vec<Vec<u8>>>,
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
        for digest in self.bearers_of.remove(key).unwrap_or_default() {
            self.bearers.remove(&digest);
        }
        Some(live)
    }

    /// A new bearer for `live`, never outliving the session, and the only
    /// current one: every bearer before it is retired, whichever way this one
    /// came. Under the lock that found `live` standing, so no bearer outlives
    /// an ended session. An admin session's lasts as long as the session: it
    /// works on the loopback listener alone, and nothing there renews it.
    fn issue(&mut self, live: &Live, now: i64) -> Issued {
        for digest in self.bearers_of.get(&live.key).into_iter().flatten() {
            if let Some(bearer) = self.bearers.get_mut(digest) {
                bearer.replaced = true;
                bearer.expires_at = bearer.expires_at.min(now + OVERLAP_SECS);
            }
        }
        let token = crate::auth::new_token();
        let digest = crate::auth::token_digest(&token);
        let lasting = live.signed().is_some_and(|signed| signed.admin);
        let expires_at = if lasting {
            live.expires_at()
        } else {
            (now + bearer_secs()).min(live.expires_at())
        };
        self.bearers_of
            .entry(live.key.clone())
            .or_default()
            .push(digest.clone());
        self.bearers.insert(
            digest,
            Bearer {
                key: live.key.clone(),
                expires_at,
                replaced: false,
            },
        );
        let renew_in = (expires_at - now - RENEW_MARGIN_SECS).max(1);
        Issued {
            token,
            expires_at,
            renew_in,
        }
    }

    /// Drops the bearers that ran out by `now`.
    fn sweep_bearers(&mut self, now: i64) {
        let Self {
            bearers,
            bearers_of,
            ..
        } = self;
        bearers.retain(|_, bearer| now < bearer.expires_at);
        bearers_of.retain(|_, digests| {
            digests.retain(|digest| bearers.contains_key(digest));
            !digests.is_empty()
        });
    }
}

/// A bearer a session was just given: the token goes to the client once and
/// only its digest stays here.
pub(crate) struct Issued {
    /// 64 lowercase hex characters.
    pub(crate) token: String,
    /// When it stops working, in Unix seconds.
    pub(crate) expires_at: i64,
    /// How many seconds from now to renew it: relative, so that a client's
    /// clock does not come into it.
    pub(crate) renew_in: i64,
}

/// The shortest and longest a bearer lives, in seconds. Drawn per bearer, so
/// that sessions begun together do not all renew at once.
const BEARER_SECS: std::ops::RangeInclusive<u16> = 64..=128;

/// How long a replaced bearer still serves requests already on their way.
const OVERLAP_SECS: i64 = 10;

/// How long before a bearer ends its client is told to renew it: room for a
/// slow network and one retry.
const RENEW_MARGIN_SECS: i64 = 20;

/// A lifetime from [`BEARER_SECS`]. The bias of reducing 16 random bits to 65
/// values is far too small to matter for spreading renewals.
fn bearer_secs() -> i64 {
    let mut bytes = [0u8; 2];
    getrandom::fill(&mut bytes).expect("OS RNG available");
    let span = BEARER_SECS.end() - BEARER_SECS.start() + 1;
    i64::from(BEARER_SECS.start() + u16::from_be_bytes(bytes) % span)
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
        signed: Signed,
    ) -> Result<Arc<Live>, Full> {
        self.open_as(key, account, expires_at, now, Some(signed))
    }

    /// [`Book::open`] for a guest or a local account, whose end the database
    /// decides: it lasts here until something ends it.
    ///
    /// # Errors
    /// [`Full`], as [`Book::open`].
    pub(crate) fn open_local(&self, key: Key, account: &str, now: i64) -> Result<Arc<Live>, Full> {
        self.open_as(key, account, i64::MAX, now, None)
    }

    fn open_as(
        &self,
        key: Key,
        account: &str,
        expires_at: i64,
        now: i64,
        signed: Option<Signed>,
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
        let local = signed.is_none();
        let live = Live::new(key.clone(), account.to_owned(), expires_at, local, signed);
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
        self.index.lock().sweep_bearers(now);
    }

    /// A new bearer for `live`, or none if it no longer stands.
    pub(crate) fn issue(&self, live: &Live, now: i64) -> Option<Issued> {
        let mut index = self.index.lock();
        let held = index.by_key.get(&live.key)?;
        if !held.stands(now) || !std::ptr::eq(Arc::as_ptr(held), live) {
            return None;
        }
        Some(index.issue(live, now))
    }

    /// The session a bearer reaches, while both stand.
    pub(crate) fn by_bearer(&self, digest: &[u8], now: i64) -> Option<Arc<Live>> {
        let index = self.index.lock();
        let bearer = index.bearers.get(digest).filter(|b| now < b.expires_at)?;
        index
            .by_key
            .get(&bearer.key)
            .filter(|live| live.stands(now))
            .cloned()
    }

    /// Replaces the bearer at `digest` with a new one. Only the current
    /// bearer renews: the old one keeps serving what is in flight for
    /// [`OVERLAP_SECS`], and asking again with it is refused, so a copy of a
    /// used bearer cannot fork a session. A client whose answer was lost
    /// presents its gateway token instead.
    pub(crate) fn renew(&self, digest: &[u8], now: i64) -> Option<Issued> {
        let mut index = self.index.lock();
        let bearer = index.bearers.get(digest).filter(|b| now < b.expires_at)?;
        if bearer.replaced {
            return None;
        }
        let live = index
            .by_key
            .get(&bearer.key)
            .filter(|live| live.stands(now))
            .cloned()?;
        Some(index.issue(&live, now))
    }

    /// Every local session something still holds, forgetting the ones nothing
    /// holds, so the book does not keep what no socket watches.
    pub(crate) fn local_in_use(&self) -> Vec<Arc<Live>> {
        let mut index = self.index.lock();
        let idle: Vec<Key> = index
            .by_key
            .iter()
            .filter(|(_, live)| live.local && Arc::strong_count(live) == 1)
            .map(|(key, _)| key.clone())
            .collect();
        for key in idle {
            index.forget(&key);
        }
        index
            .by_key
            .values()
            .filter(|live| live.local)
            .cloned()
            .collect()
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

    fn signed() -> Signed {
        Signed::new([1; 16], false, None)
    }

    #[test]
    fn a_session_stands_until_it_runs_out_even_if_no_sweep_came() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        assert!(live.stands(NOW));
        assert!(live.stands(LATER - 1));
        assert!(!live.stands(LATER), "the end itself is already too late");
        assert!(book.get(&key(1), LATER).is_none());
    }

    #[test]
    fn ending_flips_every_holder_and_keeps_the_first_reason() {
        let book = Book::default();
        let socket = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        let other = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
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
        let live = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
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
        let old = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        book.end(&key(1), Ended::Revoked);
        let new = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        assert!(!Arc::ptr_eq(&old, &new));
        assert!(!old.stands(NOW), "the old socket stays closed");
        assert!(new.stands(NOW));
    }

    #[test]
    fn a_key_never_passes_to_another_account() {
        let book = Book::default();
        let first = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        let second = book.open(key(1), "b", LATER, NOW, signed()).unwrap();
        assert!(!first.stands(NOW));
        assert_eq!(second.account(), "b");
        book.end_account("a", Ended::Banned);
        assert!(second.stands(NOW), "the old account's index is clean");
    }

    #[test]
    fn the_end_moves_later_and_never_earlier() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        book.open(key(1), "a", LATER + 100, NOW, signed()).unwrap();
        assert_eq!(live.expires_at(), LATER + 100);
        book.open(key(1), "a", LATER - 100, NOW, signed()).unwrap();
        assert_eq!(live.expires_at(), LATER + 100);
    }

    #[test]
    fn a_ban_ends_every_session_of_that_account_and_no_other() {
        let book = Book::default();
        let first = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        let second = book.open(key(2), "a", LATER, NOW, signed()).unwrap();
        let other = book.open(key(3), "b", LATER, NOW, signed()).unwrap();
        book.end_account("a", Ended::Banned);
        assert!(!first.stands(NOW) && !second.stands(NOW));
        assert!(other.stands(NOW));
        assert_eq!(book.len(), 1);
    }

    #[test]
    fn the_sweep_ends_what_ran_out_and_leaves_the_rest() {
        let book = Book::default();
        let gone = book.open(key(1), "a", NOW + 1, NOW, signed()).unwrap();
        let kept = book.open(key(2), "a", LATER, NOW, signed()).unwrap();
        book.sweep(NOW + 1);
        assert_eq!(*gone.end.borrow(), Some(Ended::Expired));
        assert!(kept.stands(NOW + 1));
        assert_eq!(book.len(), 1);
    }

    #[test]
    fn a_socket_closes_with_why_its_session_ended_and_with_nothing_while_it_stands() {
        let book = Book::default();
        let banned = book.open(key(1), "a", i64::MAX, NOW, signed()).unwrap();
        let revoked = book.open(key(2), "b", i64::MAX, NOW, signed()).unwrap();
        assert!(banned.close_frame().is_none());
        book.end(&key(1), Ended::Banned);
        book.end(&key(2), Ended::Revoked);
        let frame = banned.close_frame().unwrap();
        assert_eq!(frame.code, baylee_protocol::refusal::REFUSAL_CLOSE);
        assert_eq!(frame.reason.as_str(), "banned");
        assert_eq!(
            revoked.close_frame().unwrap().reason.as_str(),
            "session_ended"
        );
    }

    #[test]
    fn a_full_book_refuses_a_new_session_and_still_finds_the_ones_it_holds() {
        let book = Book::default();
        let live = book.open(key(0), "a", LATER, NOW, signed()).unwrap();
        {
            let mut index = book.index.lock();
            for n in 1..MAX_SESSIONS {
                let filler = n.to_be_bytes().to_vec();
                let held = Live::new(filler.clone(), "b".into(), LATER, false, None);
                index.by_key.insert(filler, held);
            }
        }
        assert_eq!(
            book.open(key(9), "c", LATER, NOW, signed()).err(),
            Some(Full)
        );
        let same = book.open(key(0), "a", LATER, NOW, signed()).unwrap();
        assert!(Arc::ptr_eq(&live, &same));
    }

    fn digest(issued: &Issued) -> Vec<u8> {
        crate::auth::token_digest(&issued.token)
    }

    #[test]
    fn a_bearer_lives_between_its_bounds_and_never_past_its_session() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        for _ in 0..512 {
            let issued = book.issue(&live, NOW).unwrap();
            assert!((NOW + 64..=NOW + 128).contains(&issued.expires_at));
        }
        let short = book.open(key(2), "a", NOW + 5, NOW, signed()).unwrap();
        assert_eq!(book.issue(&short, NOW).unwrap().expires_at, NOW + 5);
    }

    #[test]
    fn a_bearer_reaches_its_session_only_until_it_runs_out() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        let issued = book.issue(&live, NOW).unwrap();
        let found = book.by_bearer(&digest(&issued), NOW).unwrap();
        assert!(Arc::ptr_eq(&live, &found));
        assert!(
            book.by_bearer(&digest(&issued), issued.expires_at)
                .is_none()
        );
        assert!(book.by_bearer(&[0; 32], NOW).is_none());
    }

    #[test]
    fn a_renewal_keeps_the_session_and_the_old_bearer_serves_only_what_is_in_flight() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        let old = book.issue(&live, NOW).unwrap();
        let new = book.renew(&digest(&old), NOW).unwrap();
        let found = book.by_bearer(&digest(&new), NOW).unwrap();
        assert!(Arc::ptr_eq(&live, &found), "the same session, a new bearer");
        assert!(book.by_bearer(&digest(&old), NOW).is_some());
        assert!(book.by_bearer(&digest(&old), NOW + OVERLAP_SECS).is_none());
    }

    #[test]
    fn a_used_bearer_cannot_renew_again_so_a_copy_cannot_fork_the_session() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        let old = book.issue(&live, NOW).unwrap();
        assert!(book.renew(&digest(&old), NOW).is_some());
        assert!(book.renew(&digest(&old), NOW).is_none());
    }

    /// A refresh issues a bearer too; a copy of one from before it must not
    /// count as current and fork the session.
    #[test]
    fn every_new_bearer_retires_the_ones_before_it_however_it_came() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        let old = book.issue(&live, NOW).unwrap();
        let refreshed = book.issue(&live, NOW).unwrap();
        assert!(book.renew(&digest(&old), NOW).is_none(), "retired");
        assert!(
            book.by_bearer(&digest(&old), NOW).is_some(),
            "still in flight"
        );
        assert!(book.by_bearer(&digest(&old), NOW + OVERLAP_SECS).is_none());
        assert!(book.renew(&digest(&refreshed), NOW).is_some());
    }

    #[test]
    fn an_admin_bearer_lasts_as_long_as_its_session_and_a_renewal_does_not_shorten_it() {
        let book = Book::default();
        let admin = Signed::new([1; 16], true, None);
        let live = book.open(key(1), "a", LATER, NOW, admin).unwrap();
        let issued = book.issue(&live, NOW).unwrap();
        assert_eq!(issued.expires_at, LATER);
        let renewed = book.renew(&digest(&issued), NOW).unwrap();
        assert_eq!(renewed.expires_at, LATER);
    }

    #[test]
    fn an_ended_session_takes_its_bearers_with_it() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        let issued = book.issue(&live, NOW).unwrap();
        book.end(&key(1), Ended::Revoked);
        assert!(book.by_bearer(&digest(&issued), NOW).is_none());
        assert!(book.renew(&digest(&issued), NOW).is_none());
        assert!(book.index.lock().bearers.is_empty());
    }

    #[test]
    fn no_bearer_is_issued_for_an_ended_session_even_if_its_key_came_back() {
        let book = Book::default();
        let old = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        book.end(&key(1), Ended::Revoked);
        assert!(book.issue(&old, NOW).is_none(), "the key is gone");
        let new = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        assert!(
            book.issue(&old, NOW).is_none(),
            "a new session holds the key"
        );
        assert!(book.issue(&new, NOW).is_some());
    }

    #[test]
    fn ending_a_session_leaves_the_bearers_of_others_alone() {
        let book = Book::default();
        let gone = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        let kept = book.open(key(2), "b", LATER, NOW, signed()).unwrap();
        book.issue(&gone, NOW).unwrap();
        let issued = book.issue(&kept, NOW).unwrap();
        book.end(&key(1), Ended::Revoked);
        assert!(book.by_bearer(&digest(&issued), NOW).is_some());
        let index = book.index.lock();
        assert_eq!((index.bearers.len(), index.bearers_of.len()), (1, 1));
    }

    #[test]
    fn the_sweep_drops_bearers_that_ran_out_and_keeps_live_ones() {
        let book = Book::default();
        let live = book.open(key(1), "a", LATER, NOW, signed()).unwrap();
        let old = book.issue(&live, NOW).unwrap();
        let new = book.renew(&digest(&old), NOW).unwrap();
        book.sweep(NOW + OVERLAP_SECS);
        assert!(book.by_bearer(&digest(&new), NOW + OVERLAP_SECS).is_some());
        assert_eq!(book.index.lock().bearers.len(), 1);
    }

    #[test]
    fn a_local_session_lasts_until_ended_and_goes_once_nothing_holds_it() {
        let book = Book::default();
        let held = book.open_local(key(1), "a", NOW).unwrap();
        drop(book.open_local(key(2), "b", NOW).unwrap());
        assert!(held.stands(i64::MAX - 1));
        let in_use = book.local_in_use();
        assert_eq!(in_use.len(), 1);
        assert!(Arc::ptr_eq(&held, &in_use[0]));
        assert_eq!(book.len(), 1, "the one nothing held is gone");
    }

    #[test]
    fn a_central_session_is_never_taken_for_a_local_one() {
        let book = Book::default();
        drop(book.open(key(1), "a", LATER, NOW, signed()).unwrap());
        assert!(book.local_in_use().is_empty());
        assert_eq!(book.len(), 1, "it stays until it ends or runs out");
    }
}
