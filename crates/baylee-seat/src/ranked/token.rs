//! The AI runner token (baylee-central `docs/ai-players.md` §2): what the
//! runner holds instead of a password. An administrator issues one per
//! runner machine on Central's website, bound to the AI accounts it may
//! play; Central trades it for an ordinary native session of one of them.
//!
//! It comes from the operator, never from an argument, a URL, the
//! environment or a log: a file only the runner's service may read (its
//! first line, `bat_` and 64 hex digits, as the website shows it; a systemd
//! credential under `LoadCredential=`, `$CREDENTIALS_DIRECTORY/<name>`), or
//! this machine's credential store (`baylee-seat key set --key-env <name>
//! --host <central>`, the token on stdin; [`crate::keys`]). Either is read
//! afresh at each exchange, so a token the operator replaced is taken at the
//! next one, and nothing here keeps it longer than the exchange.

use anyhow::{Context as _, anyhow};
use baylee_account::{Endpoint, Token};
use baylee_client_core::llmseat::keys::KeyStore;
use std::path::PathBuf;
use std::sync::Arc;
use zeroize::Zeroizing;

/// Where the token is read from.
type TokenSource = dyn Fn() -> anyhow::Result<Token> + Send + Sync;

/// The way to the runner token: asked at each exchange. It is never kept
/// here, and nothing prints it.
#[derive(Clone)]
pub struct RunnerToken(Arc<TokenSource>);

impl std::fmt::Debug for RunnerToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RunnerToken(…)")
    }
}

impl RunnerToken {
    /// The token as `read` gives it at each exchange (a test's).
    pub fn new(read: impl Fn() -> anyhow::Result<Token> + Send + Sync + 'static) -> Self {
        Self(Arc::new(read))
    }

    /// The token on the first line of the file `path`.
    pub fn from_file(path: PathBuf) -> Self {
        Self::new(move || {
            let text =
                Zeroizing::new(std::fs::read_to_string(&path).with_context(|| {
                    format!("reading the runner token from {}", path.display())
                })?);
            parse(text.lines().next().unwrap_or_default())
                .map_err(|why| anyhow!("{}: {why}", path.display()))
        })
    }

    /// The token kept in `keys` under `name` for Central's host
    /// (`baylee-seat key set --key-env <name> --host <central>`).
    pub fn from_store(endpoint: &Endpoint, name: String, keys: Arc<dyn KeyStore>) -> Self {
        let central = endpoint.origin().to_owned();
        Self::new(move || {
            let text = crate::keys::stored_key(&*keys, &name, &central, &|_| None)
                .map(Zeroizing::new)
                .ok_or_else(|| {
                    anyhow!(
                        "no runner token in the credential store: keep it with \
                         `baylee-seat key set --key-env {name} --host {central}`"
                    )
                })?;
            parse(&text).map_err(|why| anyhow!("the credential store's {name}: {why}"))
        })
    }

    /// The token, read now.
    ///
    /// # Errors
    /// An unreadable source, or text that is no runner token.
    pub fn read(&self) -> anyhow::Result<Token> {
        (self.0)()
    }
}

/// A runner token's text form; the error never quotes the text.
fn parse(line: &str) -> anyhow::Result<Token> {
    if line.trim().is_empty() {
        return Err(anyhow!("holds no runner token"));
    }
    baylee_account::sdk::ai_runner::parse_token_text(line).map_err(|_| {
        anyhow!(
            "holds no runner token: one is `{}` and 64 lowercase hex digits, as Central's \
             website shows it",
            baylee_account::sdk::ai_runner::TOKEN_PREFIX
        )
    })
}
