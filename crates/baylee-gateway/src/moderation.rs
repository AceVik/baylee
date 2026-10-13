//! Acting on an account or a game from the admin console (beta.8,
//! `docs/protocol.md` §"Acting on an account" and §"Acting on a game"), and
//! the doors those acts close on the public side.
//!
//! - **A password reset** (`POST /admin/accounts/{id}/password`): a
//!   registered account gets a random password, answered once for the
//!   admin to hand on, and must choose its own at its next sign-in
//!   (`POST /auth/password`); every session it had is signed out, and its
//!   lobby sockets close. A game it sits in plays on: a seat runs on its
//!   seat token, not the session.
//! - **A ban** (`PUT …/ban`, lifted with `DELETE …/ban`): a reason and an
//!   end, or none for good. A registered account's sessions are signed out
//!   and its sign-in refused with the reason; a guest's session, which is
//!   the guest, stays and is refused with it. Either way its chairs,
//!   sockets and chair tickets go at once, as a deleted account's do.
//! - **A deletion** (`DELETE /admin/accounts/{id}`): what `DELETE /account`
//!   does, through [`crate::account::depart`].
//! - **Ending a game** (`POST /admin/games/{id}/close`): the engine is told
//!   (`CloseGame`), tells every seat why (`GameClosed`) and ends the game
//!   as it ends any other. Never a process killed by hand.
//!
//! Every act is an audit line on `baylee_gateway::audit`: the admin, the
//! act and the account or game, never a password, a token or a ban's
//! reason, which may name a person.

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{post, put};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::admin::{Actor, Admin, Refusal, iso};
use crate::{ErrorBody, Shared, auth, db_down, err, err_saying, store};

/// The longest ban reason, in characters.
pub(crate) const MAX_BAN_REASON: usize = 500;

/// The longest reason a game is ended with, in characters.
pub(crate) const MAX_CLOSE_REASON: usize = 200;

/// How long the gateway waits for an engine told to close its game before
/// it ends the game itself: an engine from before `CloseGame` passes over it.
const CLOSE_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// The symbols a reset password is written in, Crockford's base 32: the
/// digits and the letters but `I`, `L`, `O` and `U`, so no letter reads as
/// a digit when the password is copied by hand.
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// How a ban reads on a refusal: `{"reason", "until"}`, `until` an RFC 3339
/// moment in UTC or `null` for good.
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub(crate) struct BanBody {
    reason: String,
    until: Option<String>,
}

fn iso_secs(secs: u64) -> String {
    i64::try_from(secs)
        .ok()
        .and_then(|s| OffsetDateTime::from_unix_timestamp(s).ok())
        .map_or_else(String::new, iso)
}

/// The refusal a banned account's sign-in or session is answered: `403`,
/// the whole of it in one English sentence for a client that reads only
/// `error`, and the parts in `ban` for one that says it in its own words.
pub(crate) fn ban_refusal(ban: &store::Ban) -> Refusal {
    let until = ban.until.map(iso_secs);
    let sentence = match &until {
        Some(at) => format!("This account is banned until {at}. Reason: {}", ban.reason),
        None => format!("This account is banned. Reason: {}", ban.reason),
    };
    let (status, Json(mut body)) = err_saying(StatusCode::FORBIDDEN, sentence);
    body.ban = Some(BanBody {
        reason: ban.reason.clone(),
        until,
    });
    (status, Json(body))
}

/// The refusal a sign-in with an admin's reset password is answered:
/// `403` with `password_change: true`. The client asks for a new password
/// and sends both to `POST /auth/password`.
pub(crate) fn password_change_refusal() -> Refusal {
    let (status, Json(mut body)) = err(
        StatusCode::FORBIDDEN,
        "Your password was reset. Choose a new one to sign in.",
    );
    body.password_change = true;
    (status, Json(body))
}

/// Whether a session may be used: not while its account is banned, and not
/// while it must choose a new password (a session that slipped in between
/// the reset and the sessions' end).
///
/// # Errors
///
/// [`ban_refusal`], or `401` for a password still to be chosen.
pub(crate) fn admit(session: &store::Session) -> Result<(), Refusal> {
    if let Some(ban) = &session.ban {
        return Err(ban_refusal(ban));
    }
    if session.must_change_password {
        return Err(err(
            StatusCode::UNAUTHORIZED,
            "choose a new password: sign in again",
        ));
    }
    Ok(())
}

/// What `POST /auth/password` takes: the sign-in, and the password to have
/// from now on. No `Debug`: nothing may print it.
#[derive(Deserialize)]
pub(crate) struct NewPassword {
    /// As `POST /auth/login` takes it.
    #[serde(alias = "email")]
    username: String,
    /// The password it has now: an admin's reset one, or its own.
    password: String,
    /// The one it chose.
    #[serde(rename = "new_password")]
    chosen: String,
}

/// `POST /auth/password {username, password, new_password}`: an account
/// chooses its password, proving the one it has as a sign-in does (the
/// same tries, the same refusals). What an account an admin reset must do
/// before anything else; any account may. Every session it had is signed
/// out, and it is answered a new one, as a sign-in is.
pub(crate) async fn change_password(
    State(state): State<Shared>,
    Json(body): Json<NewPassword>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account = crate::routes::signin::proven(&state, &body.username, &body.password).await?;
    if let Some(ban) = account.ban_in_force(auth::now_secs()) {
        return Err(ban_refusal(ban));
    }
    let username = account.username.clone().unwrap_or_default();
    if !auth::valid_password(&username, &account.display_name, &body.chosen) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid password"));
    }
    if body.chosen == body.password {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "choose a password other than the one you have",
        ));
    }
    let new_password = body.chosen;
    let hash = tokio::task::spawn_blocking(move || auth::hash_password(&new_password))
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "hashing failed"))?;
    store::set_password(&state.db, &account.id, &hash)
        .await
        .map_err(|e| db_down(&e))?;
    // The sessions are gone; their lobby sockets go too. A game it sits
    // in plays on.
    let _ = state.departed.send(account.id.clone());
    let changed = store::Account {
        must_change_password: false,
        ..account
    };
    crate::routes::signin::signed_in(&state, &changed).await
}

// ------------------------------------------------------------ the console

/// The console's routes for acting on an account or a game; merged into
/// [`crate::admin`]'s router behind its guard.
pub(crate) fn console_routes() -> Router<Admin> {
    Router::new()
        .route("/admin/accounts/{id}/password", post(reset_password))
        .route("/admin/accounts/{id}/ban", put(ban).delete(unban))
        .route("/admin/games/{id}/close", post(close_game))
}

/// An account's id from a path.
fn account_id(id: &str) -> Result<String, Refusal> {
    uuid::Uuid::parse_str(id)
        .map(|id| id.to_string())
        .map_err(|_| err(StatusCode::BAD_REQUEST, "not an account's id"))
}

/// A fresh password to hand a player: 80 random bits as four groups of four
/// symbols, `7KQ2-…`.
fn reset_password_text() -> String {
    let mut bytes = [0u8; 10];
    getrandom::fill(&mut bytes).expect("OS RNG available");
    let bits = bytes
        .iter()
        .fold(0u128, |acc, b| (acc << 8) | u128::from(*b));
    let symbols: String = (0..16)
        .map(|at| char::from(ALPHABET[usize::try_from((bits >> (at * 5)) & 31).unwrap_or(0)]))
        .collect();
    symbols
        .as_bytes()
        .chunks(4)
        .map(|group| std::str::from_utf8(group).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("-")
}

fn audit(actor: &str, action: &str, account: &str) {
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action,
        account = %account,
        "admin console: an account acted on"
    );
}

/// `POST /admin/accounts/{id}/password`: a registered account's password
/// reset; `200 {"password"}`, the one time it is shown. `404` for no such
/// account, `409` for a guest's, which has none.
async fn reset_password(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, Refusal> {
    let id = account_id(&id)?;
    let app = &console.app;
    let account = store::account(&app.db, &id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such account"))?;
    if account.guest {
        return Err(err(
            StatusCode::CONFLICT,
            "a guest has no password to reset",
        ));
    }
    let password = reset_password_text();
    let plain = password.clone();
    let hash = tokio::task::spawn_blocking(move || auth::hash_password(&plain))
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "hashing failed"))?;
    if !store::reset_password(&app.db, &id, &hash)
        .await
        .map_err(|e| db_down(&e))?
    {
        return Err(err(StatusCode::NOT_FOUND, "no such account"));
    }
    // Its sessions are gone; its lobby sockets go with them.
    let _ = app.departed.send(id.clone());
    audit(&actor, "account.password.reset", &id);
    Ok(Json(serde_json::json!({ "password": password })))
}

/// What `PUT /admin/accounts/{id}/ban` takes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BanOrder {
    /// The admin's words, which the player is shown.
    reason: String,
    /// An RFC 3339 moment, or absent/`null` for good.
    #[serde(default)]
    until: Option<String>,
}

/// A ban's reason or a game's, trimmed, or why not.
fn reason_of(text: &str, max: usize) -> Result<String, Refusal> {
    let text = text.trim();
    if text.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "a reason is needed"));
    }
    if text.chars().count() > max {
        return Err(err_saying(
            StatusCode::BAD_REQUEST,
            format!("a reason is at most {max} characters"),
        ));
    }
    if text.chars().any(char::is_control) {
        return Err(err(StatusCode::BAD_REQUEST, "a reason is one line of text"));
    }
    Ok(text.to_owned())
}

/// `until` as unix seconds, in the future, or why not.
fn until_of(until: Option<&str>, now: u64) -> Result<Option<u64>, Refusal> {
    let Some(until) = until.map(str::trim).filter(|u| !u.is_empty()) else {
        return Ok(None);
    };
    let at = OffsetDateTime::parse(until, &time::format_description::well_known::Rfc3339).map_err(
        |_| {
            err(
                StatusCode::BAD_REQUEST,
                "until is an RFC 3339 moment, 2026-10-20T18:00:00Z",
            )
        },
    )?;
    let secs = u64::try_from(at.unix_timestamp()).unwrap_or(0);
    if secs <= now {
        return Err(err(StatusCode::BAD_REQUEST, "until has already passed"));
    }
    Ok(Some(secs))
}

/// `PUT /admin/accounts/{id}/ban {"reason", "until"?}`: bans an account,
/// replacing any ban it had; `200 {"ban": {"reason", "until"}}`.
async fn ban(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, Refusal> {
    let id = account_id(&id)?;
    let order: BanOrder = serde_json::from_slice(&body).map_err(|_| {
        err(
            StatusCode::BAD_REQUEST,
            "send {\"reason\": \"…\", \"until\": \"2026-10-20T18:00:00Z\" or null}",
        )
    })?;
    let reason = reason_of(&order.reason, MAX_BAN_REASON)?;
    let until = until_of(order.until.as_deref(), auth::now_secs())?;
    let app = &console.app;
    let banned = store::ban(&app.db, &id, &reason, until, &actor)
        .await
        .map_err(|e| db_down(&e))?;
    if banned == store::Banned::Nobody {
        return Err(err(StatusCode::NOT_FOUND, "no such account"));
    }
    // Out of every room and game, its sockets closed and its chair tickets
    // spent, as a deleted account's are.
    crate::account::forget_accounts(app, std::slice::from_ref(&id));
    let until = until.map(iso_secs);
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "account.ban",
        account = %id,
        guest = banned == store::Banned::Guest,
        until = %until.as_deref().unwrap_or("never"),
        "admin console: an account banned"
    );
    Ok(Json(serde_json::json!({
        "ban": { "reason": reason, "until": until }
    })))
}

/// `DELETE /admin/accounts/{id}/ban`: lifts a ban; `204`, `404` for an
/// account with none.
async fn unban(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
) -> Result<StatusCode, Refusal> {
    let id = account_id(&id)?;
    if !store::unban(&console.app.db, &id)
        .await
        .map_err(|e| db_down(&e))?
    {
        return Err(err(StatusCode::NOT_FOUND, "no banned account by that id"));
    }
    audit(&actor, "account.unban", &id);
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /admin/accounts/{id}` (routed in [`crate::admin`], beside its
/// `GET`): deletes an account and everything
/// it owns, as `DELETE /account` does (`account::depart`); `204`, `404`
/// for none.
pub(crate) async fn delete_account(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
) -> Result<StatusCode, Refusal> {
    let id = account_id(&id)?;
    let app = &console.app;
    let gone = crate::account::depart(app, store::delete_account(&app.db, &id))
        .await
        .map_err(|e| db_down(&e))?;
    if gone.accounts.is_empty() {
        return Err(err(StatusCode::NOT_FOUND, "no such account"));
    }
    audit(&actor, "account.delete", &id);
    Ok(StatusCode::NO_CONTENT)
}

/// What `POST /admin/games/{id}/close` takes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloseOrder {
    /// The admin's words, which the players are shown.
    reason: String,
}

/// `POST /admin/games/{id}/close {"reason"}`: ends a running game through
/// its engine (`CloseGame`); `202`. `404` for no such table, `409` for one
/// that is not being played. An engine that has not ended the game after
/// [`CLOSE_WAIT`] is let go and the game marked over, as a lost engine's is.
async fn close_game(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(game_id): Path<String>,
    body: Bytes,
) -> Result<StatusCode, Refusal> {
    let order: CloseOrder = serde_json::from_slice(&body)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "send {\"reason\": \"…\"}"))?;
    let reason = reason_of(&order.reason, MAX_CLOSE_REASON)?;
    let app = &console.app;
    let engine = {
        let lobby = app.lobby.lock();
        let game = lobby
            .games
            .get(&game_id)
            .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such table"))?;
        if game.state != crate::lobby::LobbyState::Playing {
            return Err(err(StatusCode::CONFLICT, "that table is not being played"));
        }
        game.engine.clone()
    };
    let told = engine.is_some_and(|link| {
        link.send(baylee_protocol::v1::Envelope {
            msg: Some(baylee_protocol::v1::envelope::Msg::CloseGame(
                baylee_protocol::v1::CloseGame {
                    game_id: game_id.clone(),
                    reason: reason.clone(),
                },
            )),
        })
        .is_ok()
    });
    if told {
        let app = app.clone();
        let game_id = game_id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(CLOSE_WAIT).await;
            let playing = app
                .lobby
                .lock()
                .games
                .get(&game_id)
                .is_some_and(|game| game.state == crate::lobby::LobbyState::Playing);
            if playing {
                tracing::warn!(game_id, "the engine did not close its game; closed here");
                crate::engine::end_game(&app, &game_id);
            }
        });
    } else {
        // No engine to tell: nothing is running the game.
        crate::engine::end_game(app, &game_id);
    }
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "game.close",
        game = %game_id,
        "admin console: a game ended"
    );
    Ok(StatusCode::ACCEPTED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reset_password_is_four_groups_a_person_can_copy() {
        let a = reset_password_text();
        let b = reset_password_text();
        assert_ne!(a, b);
        assert_eq!(a.len(), 19, "{a}");
        let groups: Vec<&str> = a.split('-').collect();
        assert_eq!(groups.len(), 4, "{a}");
        assert!(
            groups
                .iter()
                .all(|g| g.len() == 4 && g.bytes().all(|c| ALPHABET.contains(&c))),
            "{a}"
        );
        assert!(auth::valid_password("someone", "Someone", &a));
    }

    #[test]
    fn a_reason_is_one_line_of_bounded_text() {
        assert_eq!(reason_of("  spam  ", 10).ok().unwrap(), "spam");
        assert!(reason_of("   ", 10).is_err());
        assert!(reason_of(&"x".repeat(11), 10).is_err());
        assert!(reason_of(&"ü".repeat(10), 10).is_ok());
        assert!(reason_of("two\nlines", 100).is_err());
    }

    #[test]
    fn an_end_is_a_moment_still_to_come() {
        let now = 1_790_000_000;
        assert_eq!(until_of(None, now).ok(), Some(None));
        assert_eq!(until_of(Some(""), now).ok(), Some(None));
        assert_eq!(
            until_of(Some("2026-09-21T14:13:21Z"), now).ok(),
            Some(Some(now + 1))
        );
        assert!(until_of(Some("2026-09-21T14:13:20Z"), now).is_err());
        assert!(until_of(Some("tomorrow"), now).is_err());
    }

    #[test]
    fn a_ban_reads_as_one_sentence_and_its_parts() {
        let ban = store::Ban {
            reason: "cheating".into(),
            until: Some(1_790_000_000),
            at: 1,
            by: "viktor".into(),
        };
        let (status, Json(body)) = ban_refusal(&ban);
        assert_eq!(status, StatusCode::FORBIDDEN);
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(
            json["error"],
            "This account is banned until 2026-09-21T14:13:20Z. Reason: cheating"
        );
        assert_eq!(json["ban"]["reason"], "cheating");
        assert_eq!(json["ban"]["until"], "2026-09-21T14:13:20Z");
        assert!(json.get("password_change").is_none());
        let forever = store::Ban { until: None, ..ban };
        let (_, Json(body)) = ban_refusal(&forever);
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["error"], "This account is banned. Reason: cheating");
        assert!(json["ban"]["until"].is_null());
    }
}
