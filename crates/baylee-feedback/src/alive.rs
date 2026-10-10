//! `POST /client/alive`: how many people are playing offline right now
//! (`docs/feedback.md` §"Offline now"), anonymously.
//!
//! A client whose player opted in (the default, "Send anonymous usage
//! count") says, while a game it hosts itself runs, that the game is still
//! going: every [`BEAT_SECS`] it sends `{"game": "<32 hex>"}`, a random
//! value it made for that one game, held in memory and forgotten with it.
//! Not an account, not the device id direct reports use, nothing about the
//! game. The value is there only so a game is counted once rather than once
//! per beat.
//!
//! The service keeps each value with the time it was last heard, in memory
//! only, and drops it [`TTL`] later; [`Presence::count`] is how many are
//! left. Nothing is written down, nothing is logged per beat, and a restart
//! forgets all of it. The route stands on the same switch as direct
//! reports (`FEEDBACK_DIRECT_KEY`, `503` without) and on an allowance of its
//! own, roomier than theirs: a household behind one address beats once a
//! minute per game.

use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use serde::Deserialize;
use time::OffsetDateTime;

use crate::{Refusal, Shared, refuse};

/// How often a client beats while its offline game runs. The client's
/// own constant (`baylee_client_core::usage::BEAT_SECS`) must agree.
pub const BEAT_SECS: i64 = 60;

/// How long a game counts after its last beat: two beats and a margin, so
/// one lost beat does not drop it.
pub const TTL: time::Duration = time::Duration::seconds(BEAT_SECS * 2 + 30);

/// Beats one address may send per hour: a few games behind one router.
pub const PER_ADDRESS: usize = 300;

/// Beats the service takes per hour from everyone together.
pub const PER_SERVICE: usize = 120_000;

/// The most games it holds at once; past it a new one is not counted.
pub const MAX_GAMES: usize = 20_000;

/// The largest body the route reads.
pub const MAX_BODY_BYTES: usize = 256;

/// The length of a game's value: 32 hex digits, exactly.
const GAME_CHARS: usize = 32;

/// The offline games heard from in the last [`TTL`]. Memory only.
#[derive(Default)]
pub struct Presence {
    seen: HashMap<String, OffsetDateTime>,
}

impl Presence {
    fn sweep(&mut self, now: OffsetDateTime) {
        self.seen.retain(|_, at| now - *at < TTL);
    }

    /// `game` is still going at `now`.
    pub fn beat(&mut self, game: &str, now: OffsetDateTime) {
        self.sweep(now);
        if self.seen.len() >= MAX_GAMES && !self.seen.contains_key(game) {
            return;
        }
        self.seen.insert(game.to_owned(), now);
    }

    /// How many games were heard from within [`TTL`] of `now`.
    pub fn count(&mut self, now: OffsetDateTime) -> usize {
        self.sweep(now);
        self.seen.len()
    }
}

/// What a client sends: the one field, nothing else.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Beat {
    game: String,
}

/// `POST /client/alive`: `204`, or why not.
pub(crate) async fn post(
    State(shared): State<Shared>,
    peer: Option<axum::Extension<ConnectInfo<std::net::SocketAddr>>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, Refusal> {
    let Shared { state, ui } = shared;
    if state.config.direct_key().is_none() {
        return Err(refuse(
            StatusCode::SERVICE_UNAVAILABLE,
            "usage counts are not taken here",
        ));
    }
    let address = ui.address(peer.map(|axum::Extension(ConnectInfo(a))| a.ip()), &headers);
    if !ui.allow_alive(&address) {
        return Err(refuse(StatusCode::TOO_MANY_REQUESTS, "too many beats"));
    }
    if body.len() > MAX_BODY_BYTES {
        return Err(refuse(StatusCode::PAYLOAD_TOO_LARGE, "too large"));
    }
    let beat: Beat =
        serde_json::from_slice(&body).map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a beat"))?;
    let fine = beat.game.len() == GAME_CHARS
        && beat
            .game
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
    if !fine {
        return Err(refuse(StatusCode::BAD_REQUEST, "not a beat"));
    }
    ui.beat(&beat.game);
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = "0123456789abcdef0123456789abcdef";
    const TWO: &str = "fedcba9876543210fedcba9876543210";

    /// A game counts once however often it beats, and stops counting a
    /// [`TTL`] after its last beat.
    #[test]
    fn a_game_counts_once_until_its_beats_stop() {
        let mut presence = Presence::default();
        let now = OffsetDateTime::UNIX_EPOCH + time::Duration::days(20_000);
        presence.beat(ONE, now);
        presence.beat(ONE, now + time::Duration::seconds(BEAT_SECS));
        presence.beat(TWO, now + time::Duration::seconds(BEAT_SECS));
        assert_eq!(presence.count(now + time::Duration::seconds(BEAT_SECS)), 2);
        let last = now + time::Duration::seconds(BEAT_SECS);
        assert_eq!(presence.count(last + TTL - time::Duration::seconds(1)), 2);
        assert_eq!(presence.count(last + TTL), 0, "both expired");
    }

    /// One lost beat does not drop a game.
    #[test]
    fn one_lost_beat_keeps_the_game() {
        let mut presence = Presence::default();
        let now = OffsetDateTime::UNIX_EPOCH + time::Duration::days(20_000);
        presence.beat(ONE, now);
        assert_eq!(
            presence.count(now + time::Duration::seconds(2 * BEAT_SECS + 1)),
            1
        );
    }

    /// Past [`MAX_GAMES`] a new game is not counted; a known one still is.
    #[test]
    fn the_count_is_bounded() {
        let mut presence = Presence::default();
        let now = OffsetDateTime::UNIX_EPOCH + time::Duration::days(20_000);
        for n in 0..MAX_GAMES {
            presence.beat(&format!("{n:032x}"), now);
        }
        presence.beat(TWO, now);
        assert_eq!(presence.count(now), MAX_GAMES);
    }
}
