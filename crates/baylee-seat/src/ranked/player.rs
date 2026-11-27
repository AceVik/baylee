//! The AI account at Baylee Central, through its runner's sessions
//! (baylee-central `docs/ai-players.md` §2).
//!
//! The runner token ([`RunnerToken`]) is traded for an ordinary native
//! session of the account ([`Account::ai_runner`]): no password, no master
//! key, nothing restored, and everything a runner does (the terms, its
//! offers and work, admission, the relay) works as for any native session.
//! An account may hold up to four such sessions at once, and a fifth
//! exchange ends the oldest, so sessions roll over instead of being
//! replaced: the newest one ([`Player::session`]) takes the offers and
//! every new match, while an older one keeps the matches it took until
//! they end and is then signed out ([`Player::keep_alive`]).
//!
//! A session lasts thirty minutes after its last renewal and twelve hours
//! after its exchange, and Central seats an AI only through an offer whose
//! session has `drain_minutes` (up to ten hours) left before that cap. So
//! the newest session is renewed every few minutes and replaced by a fresh
//! one every [`ROLL_OVER`], well inside any drain, and at once when Central
//! will not renew it any more (revoked, or the cap reached).

use super::RunnerToken;
use anyhow::{Context as _, anyhow, bail};
use baylee_account::ranked::{Id, ProfileTarget, Request, Response};
use baylee_account::terms::TermsSubject;
use baylee_account::{Account, Endpoint, Error};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// How long the newest session takes offers before a fresh one does: two
/// hours, so that an offer's session always has more than ten hours (the
/// longest `drain_minutes` Central allows) left before its twelve.
pub const ROLL_OVER: Duration = Duration::from_hours(2);

/// One runner session of the account: its calls one at a time (they block,
/// so each runs on the blocking pool).
#[derive(Clone)]
pub struct Session(Arc<Held>);

struct Held {
    account: Mutex<Account>,
    opened: Instant,
    /// Which of the account's sessions it is, in the order they opened.
    serial: u64,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("age", &self.0.opened.elapsed())
            .finish_non_exhaustive()
    }
}

impl Session {
    /// Runs `call` on the session's account, on the blocking pool.
    ///
    /// # Errors
    /// The call's own, and [`Error::Unavailable`] for a task that did not
    /// finish.
    pub async fn call<T: Send + 'static>(
        &self,
        call: impl FnOnce(&mut Account) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        let held = Arc::clone(&self.0);
        tokio::task::spawn_blocking(move || {
            let mut account = held.account.lock().unwrap_or_else(PoisonError::into_inner);
            call(&mut account)
        })
        .await
        .unwrap_or(Err(Error::Unavailable))
    }

    /// Whether `other` is this same session.
    #[must_use]
    pub fn same(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    /// How long ago it was exchanged.
    #[must_use]
    pub fn age(&self) -> Duration {
        self.0.opened.elapsed()
    }
}

/// An AI account as its runner plays it: the account's sessions, newest
/// last, shared by every match it plays.
#[derive(Clone)]
pub struct Player(Arc<Inner>);

struct Inner {
    endpoint: Endpoint,
    account: Id,
    token: RunnerToken,
    name: String,
    sessions: Mutex<Vec<Session>>,
    /// The exchanges for a fresh session, one at a time and spaced
    /// ([`Exchanges`]).
    exchanges: tokio::sync::Mutex<Exchanges>,
}

/// When the next exchange may be made. Central takes twelve exchanges an
/// hour for an account and stops asking its store for an address after
/// thirty refused ones in an hour, which would stop every runner on the
/// machine: so a fresh session is asked for at most every
/// [`EXCHANGE_SPACING`], and after a refusal only once a wait that doubles
/// with each one, up to an hour, has passed.
#[derive(Debug, Default)]
struct Exchanges {
    next: Option<Instant>,
    refused: u32,
}

/// The least time between two exchanges of one account: well under
/// Central's twelve an hour.
pub const EXCHANGE_SPACING: Duration = Duration::from_mins(6);

impl Exchanges {
    /// The wait after `refused` refusals in a row: a minute, doubled for
    /// each one after the first, at most an hour.
    fn backoff(refused: u32) -> Duration {
        Duration::from_mins(1)
            .saturating_mul(1 << refused.saturating_sub(1).min(6))
            .min(Duration::from_hours(1))
    }
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("name", &self.0.name)
            .field("account", &Uuid::from_bytes(self.0.account))
            .finish_non_exhaustive()
    }
}

/// The account an operator names: its id, or its full public handle
/// (`KI-Haus-Expert#00ab`), read from Central's public profile.
///
/// # Errors
/// A bare name (Central looks a handle up only with its tag), a handle
/// Central does not know, or Central unreachable.
pub async fn resolve(endpoint: &Endpoint, named: &str) -> anyhow::Result<Id> {
    if let Ok(id) = Uuid::parse_str(named) {
        return Ok(*id.as_bytes());
    }
    if !named.contains('#') {
        bail!(
            "--account is the AI account's id or its whole public handle with its tag \
             (`KI-Haus-Expert#00ab`): Central looks a name up only with its tag"
        );
    }
    let (endpoint, handle) = (endpoint.clone(), named.to_owned());
    let reply = tokio::task::spawn_blocking(move || {
        baylee_account::ranked_read(
            &endpoint,
            Request::Profile {
                target: ProfileTarget::Handle(handle),
                season: None,
            },
        )
    })
    .await
    .context("the profile lookup's task")?
    .map_err(|e| anyhow!("looking up «{named}» at Central: {e}"))?;
    match reply {
        Response::Profile { account, .. } => Ok(account),
        _ => bail!("Central answered the lookup of «{named}» with something else"),
    }
}

impl Player {
    /// Trades the token for a first session of `account`, reads its public
    /// name, and accepts Central's terms in force for it where it has not
    /// yet: its offers need them, as a person's search does.
    ///
    /// # Errors
    /// A refused token (unknown, revoked, expired, not listing the account,
    /// or an account that is no AI: Central says the same for each), or
    /// Central unreachable.
    pub async fn open(endpoint: Endpoint, account: Id, token: RunnerToken) -> anyhow::Result<Self> {
        let session = exchange(&endpoint, account, &token).await?;
        let name = session
            .call(move |a| {
                match a.ranked(Request::Profile {
                    target: ProfileTarget::Account(account),
                    season: None,
                })? {
                    Response::Profile { name, .. } => Ok(name),
                    _ => Err(Error::Rejected),
                }
            })
            .await
            .map_err(|e| anyhow!("reading the AI account's public profile: {e}"))?
            .unwrap_or_else(|| Uuid::from_bytes(account).to_string());
        let player = Self(Arc::new(Inner {
            endpoint,
            account,
            token,
            name,
            sessions: Mutex::new(vec![session]),
            exchanges: tokio::sync::Mutex::new(Exchanges {
                next: Some(Instant::now() + EXCHANGE_SPACING),
                refused: 0,
            }),
        }));
        player.accept_terms().await?;
        Ok(player)
    }

    /// The account's public name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0.name
    }

    /// The account's id.
    #[must_use]
    pub fn account(&self) -> Id {
        self.0.account
    }

    /// The newest session: the one that offers and takes new matches.
    ///
    /// # Panics
    /// Never: a player holds a session for as long as any handle to it is
    /// left ([`Player::sign_out`] waits for the last).
    #[must_use]
    pub fn session(&self) -> Session {
        self.sessions()
            .last()
            .cloned()
            .expect("a player always holds a session")
    }

    fn sessions(&self) -> MutexGuard<'_, Vec<Session>> {
        self.0
            .sessions
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// What no report may carry, each with its label: the runner token, in
    /// both its forms, and the bearer of each session whose account is not
    /// busy with a call this moment (one that is holds the same kind of
    /// secret, which the others stand for). Read now; nothing is kept.
    pub(crate) fn secrets(&self) -> Vec<(&'static str, zeroize::Zeroizing<String>)> {
        let mut secrets = Vec::new();
        if let Ok(token) = self.0.token.read() {
            secrets.push((super::reports::RUNNER_TOKEN, token.hex()));
            secrets.push((super::reports::RUNNER_TOKEN, token.bearer()));
        }
        let sessions: Vec<Session> = self.sessions().clone();
        for session in sessions {
            let Ok(account) = session.0.account.try_lock() else {
                continue;
            };
            if let Some(header) = account.bearer_header() {
                let bare = header
                    .strip_prefix("Bearer ")
                    .unwrap_or(&header)
                    .to_string();
                secrets.push((
                    baylee_client_core::bugreport::Keyring::SESSION,
                    zeroize::Zeroizing::new(bare),
                ));
            }
        }
        secrets
    }

    /// Runs `call` on the newest session.
    ///
    /// # Errors
    /// As [`Session::call`].
    pub async fn call<T: Send + 'static>(
        &self,
        call: impl FnOnce(&mut Account) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        self.session().call(call).await
    }

    /// Accepts Baylee Central's terms in force for the account, if it has
    /// not: the operator runs it, and its offers need them. Says which
    /// version it accepted.
    ///
    /// # Errors
    /// Central unreachable, or terms that changed while they were read.
    pub async fn accept_terms(&self) -> anyhow::Result<()> {
        let accepted = self
            .call(|account| {
                if account.central_terms_status()?.accepted {
                    return Ok(None);
                }
                let Some(terms) = account.central_terms()? else {
                    return Ok(None);
                };
                account.accept_terms(TermsSubject::Central, &terms.version)?;
                Ok(Some(terms.version))
            })
            .await
            .map_err(|e| anyhow!("accepting Baylee Central's terms for the AI account: {e}"))?;
        if let Some(version) = accepted {
            tracing::info!(version = ?version, "accepted Baylee Central's terms in force for the AI account");
        }
        Ok(())
    }

    /// Opens a fresh session and makes it the newest; the older ones keep
    /// their matches. One exchange at a time, and only once its wait has
    /// passed ([`Exchanges`]): before then it does nothing and says `false`.
    ///
    /// # Errors
    /// A refused or failed exchange; the sessions are then as they were.
    pub async fn roll_over(&self) -> anyhow::Result<bool> {
        let mut exchanges = self.0.exchanges.lock().await;
        if exchanges.next.is_some_and(|next| Instant::now() < next) {
            return Ok(false);
        }
        match exchange(&self.0.endpoint, self.0.account, &self.0.token).await {
            Ok(fresh) => {
                exchanges.refused = 0;
                exchanges.next = Some(Instant::now() + EXCHANGE_SPACING);
                self.sessions().push(fresh);
                tracing::info!("rolled over to a fresh runner session");
                // The terms are the account's, but they may have changed since
                // the last session read them: the fresh one, which makes the
                // next offer, accepts the ones in force first.
                drop(exchanges);
                if let Err(e) = self.accept_terms().await {
                    tracing::warn!(error = %format!("{e:#}"), "the fresh session did not accept the terms; the next offer asks again");
                }
                Ok(true)
            }
            Err(e) => {
                exchanges.refused = exchanges.refused.saturating_add(1);
                exchanges.next = Some(Instant::now() + Exchanges::backoff(exchanges.refused));
                Err(e)
            }
        }
    }

    /// Every `every` (well inside a session's half hour): renews every
    /// session, rolls over once the newest is [`ROLL_OVER`] old or Central
    /// will not renew it, and signs out an older one that no match holds
    /// any more, or that Central ended. Runs until dropped.
    #[must_use]
    pub fn keep_alive(&self, every: Duration) -> super::Aborting {
        let me = self.clone();
        super::Aborting(tokio::spawn(async move {
            loop {
                tokio::time::sleep(every).await;
                me.tend().await;
            }
        }))
    }

    async fn tend(&self) {
        let all = self.sessions().clone();
        let mut ended = Vec::new();
        for session in &all {
            match session.call(|account| account.renew()).await {
                Ok(_) => {}
                Err(Error::Rejected | Error::Conflict) => ended.push(session.0.serial),
                Err(e) => tracing::info!(error = %e, "a runner session was not renewed"),
            }
        }
        let newest = all.last().map(|s| (s.0.serial, s.age()));
        drop(all);
        if let Some((serial, age)) = newest
            && (ended.contains(&serial) || age >= ROLL_OVER)
        {
            if ended.contains(&serial) {
                tracing::warn!("Central ended the newest runner session; trading the token again");
            }
            if let Err(e) = self.roll_over().await {
                tracing::warn!(error = %format!("{e:#}"), "no fresh runner session yet");
            }
        }
        // An older session goes once Central ended it, or once no match
        // holds it: the list is then its only holder.
        let gone: Vec<Session> = {
            let mut sessions = self.sessions();
            let newest = sessions.last().map(|s| s.0.serial);
            let (keep, gone): (Vec<_>, Vec<_>) =
                std::mem::take(&mut *sessions).into_iter().partition(|s| {
                    Some(s.0.serial) == newest
                        || (!ended.contains(&s.0.serial) && Arc::strong_count(&s.0) > 1)
                });
            *sessions = keep;
            gone
        };
        for session in gone {
            sign_out(session).await;
        }
    }

    /// Signs every session out at Central, once this is the last handle to
    /// the player (an ended match's task, or the keep-alive, lets go of its
    /// own a moment after it stops); one still held after five seconds is
    /// left to lapse at Central within its half hour.
    pub async fn sign_out(self) {
        let mut me = self.0;
        for _ in 0..50 {
            match Arc::try_unwrap(me) {
                Ok(inner) => {
                    let sessions = inner
                        .sessions
                        .into_inner()
                        .unwrap_or_else(PoisonError::into_inner);
                    for session in sessions {
                        sign_out(session).await;
                    }
                    return;
                }
                Err(still) => me = still,
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        tracing::info!("the runner sessions are still held; they lapse at Central");
    }
}

/// Ends `session` at Central, once no other handle to it is left; one that
/// a match still holds ends with it, when Central's cap or a revocation
/// ends it.
async fn sign_out(session: Session) {
    let _ = tokio::task::spawn_blocking(move || {
        if let Some(held) = Arc::into_inner(session.0) {
            let account = held
                .account
                .into_inner()
                .unwrap_or_else(PoisonError::into_inner);
            let _ = account.logout();
        }
    })
    .await;
}

/// The serial of the next session this process opens.
static SERIAL: AtomicU64 = AtomicU64::new(0);

/// A fresh runner session of `account`: the token read afresh, traded at
/// Central, and dropped.
async fn exchange(
    endpoint: &Endpoint,
    account: Id,
    token: &RunnerToken,
) -> anyhow::Result<Session> {
    let (endpoint, token) = (endpoint.clone(), token.clone());
    let opened = tokio::task::spawn_blocking(move || {
        let secret = token.read()?;
        Account::ai_runner(endpoint, &secret, account).map_err(|e| match e {
            Error::Rejected => anyhow!(
                "Central refused the runner token for the AI account {}: it is unknown, \
                 revoked or expired, does not list this account, or the account is no AI",
                Uuid::from_bytes(account)
            ),
            Error::Busy => anyhow!(
                "Central is busy with this runner's exchanges (twelve an hour for an account)"
            ),
            e => anyhow!("trading the runner token at Central: {e}"),
        })
    })
    .await
    .context("the exchange's task")??;
    Ok(Session(Arc::new(Held {
        account: Mutex::new(opened),
        opened: Instant::now(),
        serial: SERIAL.fetch_add(1, Ordering::Relaxed),
    })))
}
