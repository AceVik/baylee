//! A ranked AI seat's reports, sent straight to the official gateway
//! (`--reports-to <origin>`, `docs/llm-seat.md` §"Reports from ranked
//! games"). A ranked seat has no session at its host and Central's relay
//! no route for reports, so without the flag its reports stay in the
//! reports file (route `"none"`). With it, each game's reports go as one
//! report the way a client signed in nowhere sends one
//! (`POST {origin}/client/reports`, `docs/feedback.md` §"Straight from a
//! client"): no credential, under a random device id the runner keeps for
//! the account, and the gateway files it with Baylee Central marked
//! `direct`.
//!
//! The game never waits for it. The bridge seals the report and writes it
//! to the outbox (`<state>/reports-outbox/`); the runner's sender posts
//! what waits there, oldest first, and removes a report only once the
//! gateway answered that Central filed it. A limit, a gateway that does not
//! take direct reports now, or no answer keeps it, and the sender tries
//! again later, a minute at first and twice as long each time up to an
//! hour. A report the gateway refuses for what it is (`400`, `413`) moves
//! to `<state>/reports-refused/` with the answer beside it. None is ever
//! deleted unsent.

use super::{Aborting, Player, Stop, jittered};
use crate::lobby::{DirectAnswer, Lobby};
use anyhow::{Context as _, anyhow};
use baylee_client_core::bugreport::{self, Build, Keyring, Secret};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, watch};
use uuid::Uuid;
use zeroize::Zeroizing;

#[cfg(test)]
mod tests;

/// The outbox, under the runner's state directory.
pub const OUTBOX: &str = "reports-outbox";

/// Where a report the gateway refused for what it is goes, under the
/// runner's state directory.
pub const REFUSED: &str = "reports-refused";

/// The file the account's device id is kept in, under the runner's state
/// directory.
pub const DEVICE: &str = "report-device";

/// The first wait after a report could not be sent.
pub const FIRST_WAIT: Duration = Duration::from_secs(60);

/// The longest wait between two tries.
pub const LONGEST_WAIT: Duration = Duration::from_secs(3600);

/// Between two looks at the outbox while nothing waits to be tried again.
pub const LOOK_EVERY: Duration = Duration::from_secs(15);

/// The label the runner token is named by when a report would carry it.
pub(super) const RUNNER_TOKEN: &str = "runner token";

/// Where an account's reports go straight, and the outbox they wait in.
pub struct DirectReports {
    origin: String,
    lobby: Lobby,
    outbox: PathBuf,
    refused: PathBuf,
    device: String,
    build: Build,
    /// Wakes the sender when a report is queued.
    queued: Notify,
    /// One pass over the outbox at a time, so no report is sent twice.
    passing: tokio::sync::Mutex<()>,
}

impl std::fmt::Debug for DirectReports {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectReports")
            .field("origin", &self.origin)
            .field("outbox", &self.outbox)
            .finish_non_exhaustive()
    }
}

/// What one pass over the outbox came to.
#[derive(Debug, PartialEq, Eq)]
pub enum Pass {
    /// Nothing waits any more.
    Clear,
    /// A report could not be sent now, why; it and the ones after it wait.
    Later(String),
}

/// What became of one report.
#[derive(Debug, PartialEq, Eq)]
enum Sent {
    Taken(Option<String>),
    Later(String),
    Refused(String),
}

impl DirectReports {
    /// The reports of the runner whose state directory is `state` going to
    /// the official gateway at `origin`: `https://`, or `http://` to this
    /// machine, as a client holds its report address
    /// ([`bugreport::report_address`]). Makes the outbox and the refused
    /// directory, and the account's device id the first time.
    ///
    /// # Errors
    /// An origin that is neither, or a state directory it cannot write.
    pub fn open(origin: &str, state: &Path) -> anyhow::Result<Self> {
        let origin =
            bugreport::report_address(Some(origin), None, None, None).ok_or_else(|| {
                anyhow!(
                    "--reports-to {origin}: the official gateway's origin, https:// (http:// only \
                 to this machine)"
                )
            })?;
        let outbox = state.join(OUTBOX);
        let refused = state.join(REFUSED);
        for dir in [&outbox, &refused] {
            private_dir(dir)?;
        }
        let device = device(&state.join(DEVICE))?;
        Ok(Self {
            lobby: Lobby::new(&origin),
            origin,
            outbox,
            refused,
            device,
            build: Build {
                version: baylee_build::short().to_string(),
                commit: Some(baylee_build::COMMIT.to_string()).filter(|c| !c.is_empty()),
            },
            queued: Notify::new(),
            passing: tokio::sync::Mutex::new(()),
        })
    }

    /// The gateway's origin.
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// The account's device id, which the gateway files its reports under
    /// a keyed hash of.
    #[must_use]
    pub fn device(&self) -> &str {
        &self.device
    }

    /// This build, as a report names it.
    #[must_use]
    pub const fn build(&self) -> &Build {
        &self.build
    }

    /// The outbox.
    #[must_use]
    pub fn outbox(&self) -> &Path {
        &self.outbox
    }

    /// Where refused reports are kept.
    #[must_use]
    pub fn refused(&self) -> &Path {
        &self.refused
    }

    /// The reports waiting in the outbox, oldest first.
    #[must_use]
    pub fn pending(&self) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = std::fs::read_dir(&self.outbox).map_or_else(
            |_| Vec::new(),
            |entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .filter(|path| {
                        path.extension().is_some_and(|e| e == "json")
                            && path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .is_some_and(|n| !n.starts_with('.'))
                    })
                    .collect()
            },
        );
        found.sort();
        found
    }

    /// Seals `body` against `secrets` (`bugreport::seal`) and writes it to
    /// the outbox, where the sender finds it: its file.
    ///
    /// # Errors
    /// A body that holds one of `secrets` (the reason names which, never
    /// the value), or an outbox it could not write.
    pub fn queue(&self, body: &Value, secrets: &[Secret<'_>]) -> Result<PathBuf, String> {
        let sealed = bugreport::seal(body, secrets).map_err(|leaked| format!("{leaked}"))?;
        let name = format!("{}.json", Uuid::now_v7());
        let path = self.outbox.join(&name);
        let draft = self.outbox.join(format!(".{name}.part"));
        std::fs::write(&draft, sealed.as_bytes())
            .and_then(|()| std::fs::rename(&draft, &path))
            .map_err(|e| {
                let _ = std::fs::remove_file(&draft);
                format!("the outbox {}: {e}", self.outbox.display())
            })?;
        self.queued.notify_one();
        Ok(path)
    }

    /// Sends what waits in the outbox, oldest first, until one cannot go
    /// now: that one and the ones after it wait for the next pass. One pass
    /// at a time: a second waits for the first to end.
    pub async fn pass(&self) -> Pass {
        let _passing = self.passing.lock().await;
        for path in self.pending() {
            match self.send(&path).await {
                Sent::Taken(id) => {
                    if let Err(e) = std::fs::remove_file(&path) {
                        tracing::warn!(error = %e, path = %path.display(), "a sent report stays in the outbox");
                    }
                    tracing::info!(
                        report_id = id.as_deref().unwrap_or("?"),
                        "a report reached Central"
                    );
                }
                Sent::Refused(why) => {
                    tracing::warn!(why = %why, "the gateway refused a report; it is kept");
                    self.set_aside(&path, &why);
                }
                Sent::Later(why) => return Pass::Later(why),
            }
        }
        Pass::Clear
    }

    /// One report's try.
    async fn send(&self, path: &Path) -> Sent {
        let body = match std::fs::read_to_string(path) {
            Ok(body) if serde_json::from_str::<Value>(&body).is_ok() => body,
            Ok(_) => return Sent::Refused("not a report: unreadable JSON".to_string()),
            Err(e) => return Sent::Later(format!("could not read {}: {e}", path.display())),
        };
        match self.lobby.direct_report(body).await {
            Ok(answer) => judged(&answer),
            Err(e) => Sent::Later(format!("{e:#}")),
        }
    }

    /// Moves a refused report out of the outbox, its answer beside it.
    fn set_aside(&self, path: &Path, why: &str) {
        let Some(name) = path.file_name() else {
            return;
        };
        let kept = self.refused.join(name);
        let moved = std::fs::rename(path, &kept).and_then(|()| {
            std::fs::write(kept.with_extension("why"), format!("{why}\n").as_bytes())
        });
        if let Err(e) = moved {
            tracing::warn!(error = %e, path = %path.display(), "a refused report stays in the outbox");
        }
    }

    /// Sends what waits now and whenever a report is queued, and again
    /// after a wait where one could not go, until `stop` says leave. Runs
    /// until dropped.
    #[must_use]
    pub fn deliver(self: &Arc<Self>, mut stop: watch::Receiver<Stop>) -> Aborting {
        let reports = Arc::clone(self);
        Aborting(tokio::spawn(async move {
            let mut backoff: Option<Duration> = None;
            loop {
                let next = match reports.pass().await {
                    Pass::Clear => {
                        backoff = None;
                        LOOK_EVERY
                    }
                    Pass::Later(why) => {
                        let wait = backoff.map_or(FIRST_WAIT, |w| (w * 2).min(LONGEST_WAIT));
                        backoff = Some(wait);
                        tracing::info!(
                            why = %why,
                            waiting = reports.pending().len(),
                            again_in_s = wait.as_secs(),
                            "the reports wait to be sent"
                        );
                        jittered(wait)
                    }
                };
                tokio::select! {
                    () = tokio::time::sleep(next) => {}
                    () = reports.queued.notified(), if backoff.is_none() => {}
                    changed = stop.changed() => {
                        if changed.is_err() || *stop.borrow() == Stop::Leave {
                            return;
                        }
                    }
                }
            }
        }))
    }

    /// One pass, for at most `limit`: before the runner ends, so the last
    /// game's reports go now rather than at the next start.
    pub async fn flush(&self, limit: Duration) -> Pass {
        tokio::time::timeout(limit, self.pass())
            .await
            .unwrap_or_else(|_| Pass::Later(format!("not all sent within {} s", limit.as_secs())))
    }
}

/// What a gateway's answer means for the report: Central filed it; the
/// report itself is refused (it would be again); or it may go later (a
/// limit, a gateway that does not take direct reports now, Central not
/// answering, anything else).
fn judged(answer: &DirectAnswer) -> Sent {
    let said = answer.error.as_deref().unwrap_or("no reason given");
    match answer.status {
        200..=299 => Sent::Taken(answer.report_id.clone()),
        400 | 413 | 422 => Sent::Refused(format!("{}: {said}", answer.status)),
        status => Sent::Later(format!("the gateway answered {status} ({said})")),
    }
}

/// A directory only the runner's user may read.
fn private_dir(dir: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
            .with_context(|| format!("restrict {}", dir.display()))?;
    }
    Ok(())
}

/// The account's device id, kept at `path`: the one kept there, else one
/// made now of sixteen random bytes and kept. Never derived from the
/// account or the machine; the gateway files the reports under its keyed
/// hash, never the id ([`bugreport::kept_device_id`]).
fn device(path: &Path) -> anyhow::Result<String> {
    let kept = std::fs::read_to_string(path).ok();
    let id = bugreport::kept_device_id(kept.as_deref().map(str::trim), random);
    if kept.as_deref().map(str::trim) != Some(id.as_str()) {
        std::fs::write(path, format!("{id}\n"))
            .with_context(|| format!("keep {}", path.display()))?;
    }
    Ok(id)
}

/// Sixteen bytes from the platform's generator: a v4 id's, 122 of them
/// random, as the seat names its unguessable directories.
fn random() -> [u8; 16] {
    *Uuid::new_v4().as_bytes()
}

/// What the link of one match hands the bridge to file its game's reports
/// straight ([`crate::link::Reporter::Direct`]): where they go, and what
/// the report says of the AI and its match beside the seat's own fields.
pub struct DirectFiling {
    reports: Arc<DirectReports>,
    /// The account, whose runner token and sessions no report may carry;
    /// `None` only in a test of the bridge, which has no Central.
    player: Option<Player>,
    about: Map<String, Value>,
}

impl DirectFiling {
    /// `about` is added to the report's `client`
    /// ([`crate::reports::direct`]).
    #[must_use]
    pub fn new(reports: Arc<DirectReports>, player: Player, about: Map<String, Value>) -> Self {
        Self {
            reports,
            player: Some(player),
            about,
        }
    }

    /// The same for a test of the bridge, without an account.
    #[cfg(test)]
    pub(crate) fn without_account(reports: Arc<DirectReports>, about: Map<String, Value>) -> Self {
        Self {
            reports,
            player: None,
            about,
        }
    }

    /// What the report says of the AI and its match.
    #[must_use]
    pub const fn about(&self) -> &Map<String, Value> {
        &self.about
    }

    /// The account's device id.
    #[must_use]
    pub fn device(&self) -> &str {
        self.reports.device()
    }

    /// This build.
    #[must_use]
    pub fn build(&self) -> &Build {
        self.reports.build()
    }

    /// The gateway's origin.
    #[must_use]
    pub fn origin(&self) -> &str {
        &self.reports.origin
    }

    /// Seals `body` against the seat's own secret, the runner token and the
    /// account's Central sessions, and queues it.
    ///
    /// # Errors
    /// See [`DirectReports::queue`].
    pub fn queue(&self, body: &Value, seat_secret: &str) -> Result<PathBuf, String> {
        let held: Vec<(&'static str, Zeroizing<String>)> =
            self.player.as_ref().map_or_else(Vec::new, Player::secrets);
        let mut secrets = vec![Secret {
            label: Keyring::SEAT,
            value: seat_secret,
        }];
        secrets.extend(held.iter().map(|(label, value)| Secret {
            label,
            value: value.as_str(),
        }));
        self.reports.queue(body, &secrets)
    }
}
