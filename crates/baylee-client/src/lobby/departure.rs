//! Quitting says so (`POST /lobby/depart`, `docs/protocol.md` §"Leaving,
//! and losing the connection").
//!
//! A client that just goes away is waited for: a waiting room keeps its
//! chair for a minute, a running game its seat for the reconnect window (or
//! paused, alone with the house). A player who quits on purpose is not
//! coming back, so the client tells the gateway on the way out, and the
//! rooms and tables it sat at act at once. A crash says nothing and falls
//! back to the waiting; so does a restart into an update, which is coming
//! straight back ([`restarting`]).
//!
//! A `Drop` and not a system reading `AppExit`, for the reason the updater
//! gives (`update::native`): on macOS "Quit" never sends one, and all Bevy
//! gets to do is clear its world, which drops this resource. Natively only:
//! a browser tab that closes takes its requests with it.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Set when the client quits to restart (into an update, or as a moved
/// copy): it is coming back, and says nothing.
static RESTARTING: AtomicBool = AtomicBool::new(false);

/// The longest the quit waits for the gateway to hear it. Past this the
/// client goes anyway and the gateway's own wait takes over.
const DEPART_WAIT: Duration = Duration::from_secs(2);

/// The client is quitting to start again at once: no departure.
pub(crate) fn restarting() {
    RESTARTING.store(true, Ordering::Relaxed);
}

/// Who to tell, kept current while the client runs.
#[derive(Resource, Default)]
pub(super) struct Departure {
    /// The gateway and the session signed in there, if any.
    signed_in: Option<(String, String)>,
}

/// Keeps [`Departure`] pointed at the gateway and session of the moment.
pub(super) fn track(state: Res<LobbyState>, mut departure: ResMut<Departure>) {
    let now = state
        .lobby
        .token()
        .map(|token| (state.gateway.clone(), token.to_string()));
    if departure.signed_in != now {
        departure.signed_in = now;
    }
}

impl Drop for Departure {
    fn drop(&mut self) {
        if std::thread::panicking() || RESTARTING.load(Ordering::Relaxed) {
            return;
        }
        let Some((gateway, token)) = self.signed_in.take() else {
            return;
        };
        let url = format!("{}/lobby/depart", gateway.trim_end_matches('/'));
        let request = super::http::bearer(
            super::http::json_post(&url, &serde_json::json!({})),
            Some(&token),
        );
        let (said, heard) = std::sync::mpsc::channel();
        let sent = std::thread::Builder::new()
            .name("baylee-depart".to_owned())
            .spawn(move || {
                let _ = said.send(crate::transport::fetch_blocking(&request));
            });
        if sent.is_err() {
            return;
        }
        match heard.recv_timeout(DEPART_WAIT) {
            Ok(Ok(answer)) => info!("told the gateway this client quit: {}", answer.status),
            Ok(Err(why)) => info!("could not tell the gateway this client quit: {why}"),
            Err(_) => info!("the gateway did not hear this client quit in time"),
        }
    }
}
