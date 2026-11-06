//! Keeping up with Baylee Central: its keys, and every revocation
//! (`docs/central-login.md` §"Sessions"). Pushes are the fast way, and nothing
//! depends on them: this asks on its own every 30 seconds, sooner when the
//! heartbeat or a gap says it is behind, so a gateway Central cannot reach
//! still learns within seconds.
//!
//! Every gateway asks, whether it beats or not: a token may hold `sso` without
//! `listing`, and then its beats are refused while it still signs players in.
#![forbid(unsafe_code)]

use super::{Bridge, Shared, routes, trust::Refused};
use crate::sessions::{self, Ended, Live};
use baylee_account::wire;
use std::{sync::Arc, time::Duration};

/// How often this asks with nothing to say it is behind.
const EVERY: Duration = Duration::from_secs(30);
/// How soon it asks again while it has not caught up since it started: Central's
/// accounts wait on this.
const UNTIL_CAUGHT_UP: Duration = Duration::from_secs(5);
/// The most pages one catch-up reads. Far past what two hours of revocations at
/// one gateway come to; a Central that keeps answering is not followed for ever.
const MAX_PAGES: usize = 4096;
/// Where Central sends a revocation, on this gateway's approved origin.
const INVALIDATE_PATH: &str = "/auth/central/invalidate";

/// Reads what was missed, page by page. `false` when it could not finish, which
/// ends nothing: an error from Central is no revocation.
async fn catch_up(state: &Shared, bridge: &Arc<Bridge>) -> bool {
    for _ in 0..MAX_PAGES {
        let after = bridge.trust.applied();
        let asked = wire::Request::GatewayRevocations { after };
        let Ok(wire::Response::GatewayRevocations { token, last, floor }) =
            bridge.exchange(asked).await
        else {
            return false;
        };
        // Numbers this gateway never saw are gone from Central, so it cannot
        // say whom they ended: it ends all it holds, and clients present fresh
        // tokens, which Central gives only to live sessions. After a start it
        // holds nothing, and Central holds every number still worth knowing.
        if after.saturating_add(1) < floor {
            if after > 0 {
                state.sessions.end_all(Live::is_central, Ended::Revoked);
            }
            bridge.trust.skip_to(floor.saturating_sub(1));
        }
        let Some(token) = token else {
            return bridge.trust.applied() >= last;
        };
        if !apply(state, bridge, token.expose()).await {
            return false;
        }
        if bridge.trust.applied() <= after {
            return false;
        }
    }
    false
}

/// Takes one page, fetching the keys once if it names one not held.
async fn apply(state: &Shared, bridge: &Arc<Bridge>, token: &str) -> bool {
    let mut taken = bridge.trust.fetched(token, sessions::now());
    if matches!(taken, Err(Refused::UnknownKey)) {
        bridge.refresh_keys(true).await;
        taken = bridge.trust.fetched(token, sessions::now());
    }
    let Ok(applied) = taken else {
        return false;
    };
    routes::ended(state, bridge, &applied);
    true
}

/// Names the invalidate route until Central has taken it. It blocks nothing: a
/// gateway Central does not push to catches up instead.
async fn register(bridge: &Arc<Bridge>) {
    if bridge.registered() {
        return;
    }
    let path = INVALIDATE_PATH.to_owned();
    if matches!(
        bridge.exchange(wire::Request::RegisterInvalidate { path }).await,
        Ok(wire::Response::Done)
    ) {
        bridge.mark_registered();
    }
}

/// Runs for the life of the process, with whichever link is in use.
pub(crate) fn spawn(state: Shared) {
    tokio::spawn(async move {
        loop {
            let Some(bridge) = state.central.get() else {
                tokio::time::sleep(UNTIL_CAUGHT_UP).await;
                continue;
            };
            bridge.refresh_keys(false).await;
            register(&bridge).await;
            if bridge.trust.has_keys() && catch_up(&state, &bridge).await {
                bridge.trust.caught_up();
            }
            let wait = if bridge.trust.is_ready() {
                EVERY
            } else {
                UNTIL_CAUGHT_UP
            };
            tokio::select! {
                () = tokio::time::sleep(wait) => {}
                () = bridge.sync.notified() => {}
            }
        }
    });
}
