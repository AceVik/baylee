//! The AI account, signed in to Baylee Central as the native client signs
//! in ([`baylee_account::Account::login`]), and kept signed in.
//!
//! Its password comes from the operator, never from an argument, a URL, the
//! environment or a log: a credential file only the runner's service may
//! read (systemd's `LoadCredential=`, as Central's design for AI accounts
//! has it: baylee-central `docs/ai-players.md` §2), or this machine's
//! credential store (`baylee-seat key set --key-env <name> --host
//! <central>`, [`crate::keys`]).
//!
//! One Central session serves every match of the account: a second sign-in
//! ends the first, and with it the account's seats at their hosts. It
//! expires 30 minutes after its last renewal and twelve hours after the
//! sign-in, so [`Player::keep_alive`] renews it well inside the half hour,
//! the runner signs in again ([`Player::sign_in_again`]) at a moment it
//! plays no match once the session is old ([`Player::age`]), and the
//! keep-alive signs in again when Central will not renew it any more.

use crate::mind::Disclosure;
use anyhow::{Context as _, anyhow, bail};
use baylee_account::{Account, Endpoint, Error};
use baylee_client_core::llmseat::keys::KeyStore;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;

/// The prefix Central reserves for AI accounts' public names
/// (baylee-central `docs/ai-players.md` §6: `KI-Haus-Expert`).
pub const AI_NAME_PREFIX: &str = "KI-";

/// How the runner opens an AI account's Central session: the one seam
/// between the runner and the account's credential. Today's is the
/// account's password ([`Credentials`]); Central's design replaces it with
/// an AI runner token an administrator issues per runner machine, which
/// Central exchanges for an ordinary native session of each account it may
/// play. After the sign-in, everything runs as for a native client.
pub trait SignIn: Send + Sync + 'static {
    /// A fresh native session for the account (blocking).
    ///
    /// # Errors
    /// A refused or failed sign-in.
    fn sign_in(&self) -> anyhow::Result<Account>;

    /// How long one session may serve before the runner opens a new one at
    /// a moment the account plays no match; `None` where a new session does
    /// not end the old one's matches and the keep-alive is enough.
    fn session_age(&self) -> Option<Duration>;
}

/// A password session's age before the runner signs in again: Central ends
/// it twelve hours after the sign-in, and a new sign-in ends the old one's
/// seats, so it is opened anew while no match runs, well before.
pub const PASSWORD_SESSION_AGE: Duration = Duration::from_hours(8);

/// Where the password comes from: asked each time the account signs in.
type PasswordSource = dyn Fn() -> anyhow::Result<Zeroizing<String>> + Send + Sync;

/// What the account signs in with. Its password is never kept here, only
/// the way to ask for it, and nothing prints it.
pub struct Credentials {
    endpoint: Endpoint,
    username: String,
    password: Box<PasswordSource>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("central", &self.endpoint.origin())
            .field("username", &self.username)
            .finish_non_exhaustive()
    }
}

impl Credentials {
    /// `username` at `endpoint`, its password asked of `password` at each
    /// sign-in (a test's synthetic account, or the operator's file).
    pub fn new(
        endpoint: Endpoint,
        username: String,
        password: impl Fn() -> anyhow::Result<Zeroizing<String>> + Send + Sync + 'static,
    ) -> Self {
        Self {
            endpoint,
            username,
            password: Box::new(password),
        }
    }

    /// The password in the file `path` (its first line; a systemd
    /// credential, `$CREDENTIALS_DIRECTORY/<name>`), read afresh at each
    /// sign-in.
    pub fn from_file(endpoint: Endpoint, username: String, path: PathBuf) -> Self {
        Self::new(endpoint, username, move || {
            let text = Zeroizing::new(std::fs::read_to_string(&path).with_context(|| {
                format!("reading the account's password from {}", path.display())
            })?);
            let line = text.lines().next().unwrap_or_default().trim_end();
            if line.is_empty() {
                bail!("{} holds no password", path.display());
            }
            Ok(Zeroizing::new(line.to_owned()))
        })
    }

    /// The password kept in `keys` under `name` for Central's host
    /// (`baylee-seat key set --key-env <name> --host <central>`).
    pub fn from_store(
        endpoint: Endpoint,
        username: String,
        name: String,
        keys: Arc<dyn KeyStore>,
    ) -> Self {
        let central = endpoint.origin().to_owned();
        Self::new(endpoint, username, move || {
            crate::keys::stored_key(&*keys, &name, &central, &|_| None)
                .map(Zeroizing::new)
                .ok_or_else(|| {
                    anyhow!(
                        "no password for the account in the credential store: keep it with \
                         `baylee-seat key set --key-env {name} --host {central}`"
                    )
                })
        })
    }

    /// The Central the account signs in to.
    #[must_use]
    pub const fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    fn password_sign_in(&self) -> anyhow::Result<Account> {
        let password = (self.password)()?;
        Account::login(
            self.endpoint.clone(),
            self.username.clone(),
            password.to_string(),
            Vec::new(),
        )
        .map_err(|e| anyhow!("signing in to Baylee Central as «{}»: {e}", self.username))
    }
}

impl SignIn for Credentials {
    fn sign_in(&self) -> anyhow::Result<Account> {
        self.password_sign_in()
    }

    fn session_age(&self) -> Option<Duration> {
        Some(PASSWORD_SESSION_AGE)
    }
}

/// A signed-in AI account, shared by every match it plays: one Central
/// session, its calls one at a time (they block, so each runs on the
/// blocking pool).
#[derive(Clone)]
pub struct Player(Arc<Inner>);

struct Inner {
    credentials: Box<dyn SignIn>,
    account: Mutex<(Account, Instant)>,
    name: String,
}

impl std::fmt::Debug for Player {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("name", &self.0.name)
            .finish_non_exhaustive()
    }
}

/// Whether `name` says it is an AI played by `disclosure`'s kind of mind:
/// Central's reserved prefix (`KI-Haus-Expert`) or the mind's own
/// (`HOUSE-Expert`), as a lobby chair is named.
#[must_use]
pub fn labelled(name: &str, disclosure: Disclosure) -> bool {
    disclosure.names(name)
        || name
            .strip_prefix(AI_NAME_PREFIX)
            .is_some_and(|rest| !rest.is_empty())
}

impl Player {
    /// Signs in, and holds the account to its label ([`labelled`]): a
    /// ranked host names its chairs itself, so no AI label of the seat's
    /// own reaches the table (`crate::Labelled::Account`). Once Central
    /// flags AI accounts, its flag is the label.
    ///
    /// # Errors
    /// A failed sign-in, an unreadable profile, or a name without the label.
    pub async fn sign_in(credentials: impl SignIn, disclosure: Disclosure) -> anyhow::Result<Self> {
        let credentials: Box<dyn SignIn> = Box::new(credentials);
        let (credentials, account, name) = tokio::task::spawn_blocking(move || {
            let account = credentials.sign_in()?;
            let handle = account
                .profile_handle()
                .ok_or_else(|| anyhow!("Central did not say the account's public name"))?;
            Ok::<_, anyhow::Error>((credentials, account, public_name(&handle).to_owned()))
        })
        .await
        .context("the sign-in task")??;
        if !labelled(&name, disclosure) {
            let _ = tokio::task::spawn_blocking(move || account.logout()).await;
            bail!(
                "the account «{name}» does not say it is an AI: its public name must \
                 start with {AI_NAME_PREFIX} or {}",
                disclosure.prefix()
            );
        }
        Ok(Self(Arc::new(Inner {
            credentials,
            account: Mutex::new((account, Instant::now())),
            name,
        })))
    }

    /// The account's public name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.0.name
    }

    /// Whether the session is old enough to be opened anew while the
    /// account plays no match ([`SignIn::session_age`]).
    #[must_use]
    pub fn session_is_old(&self) -> bool {
        self.0
            .credentials
            .session_age()
            .is_some_and(|age| self.age() >= age)
    }

    /// How long ago the account's session was opened.
    #[must_use]
    pub fn age(&self) -> Duration {
        self.0
            .account
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .1
            .elapsed()
    }

    /// Runs `call` on the account, on the blocking pool.
    ///
    /// # Errors
    /// The call's own, and [`Error::Unavailable`] for a task that did not
    /// finish.
    pub async fn call<T: Send + 'static>(
        &self,
        call: impl FnOnce(&mut Account) -> Result<T, Error> + Send + 'static,
    ) -> Result<T, Error> {
        let inner = Arc::clone(&self.0);
        tokio::task::spawn_blocking(move || {
            let mut held = inner.account.lock().unwrap_or_else(PoisonError::into_inner);
            call(&mut held.0)
        })
        .await
        .unwrap_or(Err(Error::Unavailable))
    }

    /// Renews the Central session every `every` (well inside its half
    /// hour), and signs in again when Central will not renew it any more
    /// (the twelve hours are up, or it was revoked). Runs until dropped.
    #[must_use]
    pub fn keep_alive(&self, every: Duration) -> super::Aborting {
        let me = self.clone();
        super::Aborting(tokio::spawn(async move {
            loop {
                tokio::time::sleep(every).await;
                match me.call(|account| account.renew()).await {
                    Ok(_) => {}
                    Err(Error::Rejected | Error::Conflict) => {
                        tracing::warn!("the Central session ended; signing in again");
                        if let Err(e) = me.sign_in_again().await {
                            tracing::warn!(error = %format!("{e:#}"), "signing in again failed");
                        }
                    }
                    Err(e) => tracing::info!(error = %e, "the Central session was not renewed"),
                }
            }
        }))
    }

    /// Opens a new Central session in place of the old one, reading the
    /// password afresh. The old session ends, and with it the account's
    /// seats at their hosts: call it only while the account plays no match.
    ///
    /// # Errors
    /// A failed sign-in; the old session is then kept.
    pub async fn sign_in_again(&self) -> anyhow::Result<()> {
        let inner = Arc::clone(&self.0);
        tokio::task::spawn_blocking(move || {
            let fresh = inner.credentials.sign_in()?;
            let (old, _) = std::mem::replace(
                &mut *inner.account.lock().unwrap_or_else(PoisonError::into_inner),
                (fresh, Instant::now()),
            );
            // Already ended by the new sign-in; its keys are dropped here.
            let _ = old.logout();
            Ok(())
        })
        .await
        .context("the sign-in task")?
    }

    /// Signs out at Central, once no other handle to the account is left.
    pub async fn sign_out(self) {
        let inner = self.0;
        let _ = tokio::task::spawn_blocking(move || {
            if let Some(inner) = Arc::into_inner(inner) {
                let (account, _) = inner
                    .account
                    .into_inner()
                    .unwrap_or_else(PoisonError::into_inner);
                let _ = account.logout();
            }
        })
        .await;
    }
}

/// The name in a public handle (`Name#00ab` → `Name`).
pub(super) fn public_name(handle: &str) -> &str {
    handle.rsplit_once('#').map_or(handle, |(name, _)| name)
}
