//! The admin pages' acts on an account or a game (beta.8, `docs/feedback.md`
//! §"The admin console"), passed on to the gateway's console as the reads
//! are ([`crate::console`]), for a signed-in admin with the UI's CSRF proof:
//!
//! - `PUT /ui/api/admin/accounts/{id}/ban` `{reason, until?}`: a ban;
//!   `DELETE` the same path lifts it.
//! - `DELETE /ui/api/admin/accounts/{id}`: the account deleted, as its
//!   player's own deletion does.
//! - `POST /ui/api/admin/games/{id}/close` `{reason}`: a running game
//!   ended through its engine.
//!
//! Each is written into `feedback_audit` with the admin and the account or
//! game, never a reason, which may name a person. There is no password to
//! reset: accounts sign in at Baylee Central (`docs/central-login.md`).

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Response;
use axum::routing::{post, put};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::console::{answer, ask, note, signed_in};
use crate::ui::same_origin;
use crate::{Refusal, Shared, refuse};

pub(crate) fn routes() -> Router<Shared> {
    Router::new()
        .route("/ui/api/admin/accounts/{id}/ban", put(ban).delete(unban))
        .route("/ui/api/admin/games/{id}/close", post(close_game))
}

fn account_id(id: &str) -> Result<Uuid, Refusal> {
    Uuid::parse_str(id).map_err(|_| refuse(StatusCode::BAD_REQUEST, "not an account's id"))
}

/// The audit row for an act that worked, beside a log line. A row that
/// cannot be written is logged and the act still answered: it is done at
/// the gateway, whose own audit line has it.
async fn noted(shared: &Shared, admin: &str, action: &str, detail: &str) {
    tracing::info!(admin, action, %detail, "admin console");
    if let Err(e) = note(shared, admin, action, detail).await {
        tracing::error!("admin console: the audit row was not written: {e}");
    }
}

/// What `PUT /ui/api/admin/accounts/{id}/ban` takes, and all it passes on.
///
/// @description A ban for an account.
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BanOrder {
    /// @description Why, as the player is told it.
    reason: String,
    /// @description When the ban ends, RFC 3339; absent for never.
    /// @openapi format: date-time
    #[serde(default, skip_serializing_if = "Option::is_none")]
    until: Option<String>,
}

/// The audit's words for a ban: the account and its end, never the reason.
fn ban_detail(id: Uuid, until: Option<&str>) -> String {
    format!("account {id}, until {}", until.unwrap_or("never"))
}

/// @route PUT /ui/api/admin/accounts/{id}/ban
/// @scope feedback
/// @summary Ban an account
/// @description The gateway's `PUT /admin/accounts/{id}/ban`, passed on
/// with its status. A ban made is an audit row with the account and the
/// ban's end, never the reason.
/// @tag Moderation
/// @security session + csrf
/// @path-param id: String format=uuid "The account's id"
/// @body `$BanOrder`
/// @insert `ConsoleChangeRefusals`
/// @return 200: {} "Banned, as the gateway answers: the account's `ban` with its end"
/// @return 400: `$ErrorBody` "Not an account's id, not a ban, or the gateway's own refusal of its reason or end"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such account"
async fn ban(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let id = account_id(&id)?;
    let order: BanOrder =
        serde_json::from_slice(&body).map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a ban"))?;
    let body =
        serde_json::to_vec(&order).map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a ban"))?;
    let done = ask(
        &gateway,
        "PUT",
        format!("/admin/accounts/{id}/ban"),
        &admin,
        Some(body),
    )
    .await?;
    if done.status == 200 {
        let until = done.json["ban"]["until"].as_str().map(str::to_owned);
        noted(
            &shared,
            &admin,
            "gateway.account.ban",
            &ban_detail(id, until.as_deref()),
        )
        .await;
    }
    Ok(answer(done))
}

/// @route DELETE /ui/api/admin/accounts/{id}/ban
/// @scope feedback
/// @summary Lift an account's ban
/// @description The gateway's `DELETE /admin/accounts/{id}/ban`, passed on
/// with its status; a ban lifted is an audit row.
/// @tag Moderation
/// @security session + csrf
/// @path-param id: String format=uuid "The account's id"
/// @insert `ConsoleChangeRefusals`
/// @return 204: "Lifted"
/// @return 400: `$ErrorBody` "Not an account's id"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such account or no ban on it"
async fn unban(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let id = account_id(&id)?;
    let done = ask(
        &gateway,
        "DELETE",
        format!("/admin/accounts/{id}/ban"),
        &admin,
        None,
    )
    .await?;
    if done.status == 204 {
        noted(
            &shared,
            &admin,
            "gateway.account.unban",
            &format!("account {id}"),
        )
        .await;
    }
    Ok(answer(done))
}

/// `DELETE /ui/api/admin/accounts/{id}` (routed in [`crate::console`],
/// beside its `GET`).
///
/// @route DELETE /ui/api/admin/accounts/{id}
/// @scope feedback
/// @summary Delete an account
/// @description The gateway's `DELETE /admin/accounts/{id}`, passed on with
/// its status: the account is deleted as its player's own deletion does.
/// An account deleted is an audit row.
/// @tag Moderation
/// @security session + csrf
/// @path-param id: String format=uuid "The account's id"
/// @insert `ConsoleChangeRefusals`
/// @return 204: "Deleted"
/// @return 400: `$ErrorBody` "Not an account's id"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such account"
pub(crate) async fn delete_account(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let id = account_id(&id)?;
    let done = ask(
        &gateway,
        "DELETE",
        format!("/admin/accounts/{id}"),
        &admin,
        None,
    )
    .await?;
    if done.status == 204 {
        noted(
            &shared,
            &admin,
            "gateway.account.delete",
            &format!("account {id}"),
        )
        .await;
    }
    Ok(answer(done))
}

/// What `POST /ui/api/admin/games/{id}/close` takes, and all it passes on.
///
/// @description Why a game is ended.
/// @openapi
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CloseOrder {
    /// @description Why, as the players are told it.
    reason: String,
}

/// @route POST /ui/api/admin/games/{id}/close
/// @scope feedback
/// @summary End a running game
/// @description The gateway's `POST /admin/games/{id}/close`, passed on
/// with its status: the gateway ends the game through its engine. A game
/// ended is an audit row, never with its reason.
/// @tag Moderation
/// @security session + csrf
/// @path-param id: String format=uuid "The game's id"
/// @body `$CloseOrder`
/// @insert `ConsoleChangeRefusals`
/// @return 202: "Asked; the engine ends the game"
/// @return 400: `$ErrorBody` "Not a game's id, not a reason, or the gateway's own refusal of it"
/// @return 404: `$ErrorBody` "No gateway console is configured, or the gateway has no such table"
/// @return 409: `$ErrorBody` "That table is not being played (the gateway's)"
async fn close_game(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let id =
        Uuid::parse_str(&id).map_err(|_| refuse(StatusCode::BAD_REQUEST, "not a game's id"))?;
    let order: CloseOrder = serde_json::from_slice(&body)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "send {\"reason\": \"…\"}"))?;
    let body = serde_json::to_vec(&order)
        .map_err(|_| refuse(StatusCode::BAD_REQUEST, "send {\"reason\": \"…\"}"))?;
    let done = ask(
        &gateway,
        "POST",
        format!("/admin/games/{id}/close"),
        &admin,
        Some(body),
    )
    .await?;
    if done.status == 202 {
        noted(&shared, &admin, "gateway.game.close", &format!("game {id}")).await;
    }
    Ok(answer(done))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ban_is_audited_by_its_account_and_end_never_its_reason() {
        let id = Uuid::nil();
        assert_eq!(
            ban_detail(id, Some("2026-10-20T18:00:00Z")),
            "account 00000000-0000-0000-0000-000000000000, until 2026-10-20T18:00:00Z"
        );
        assert_eq!(
            ban_detail(id, None),
            "account 00000000-0000-0000-0000-000000000000, until never"
        );
    }

    #[test]
    fn a_ban_passes_on_its_two_fields_and_nothing_else() {
        assert!(serde_json::from_str::<BanOrder>(r#"{"reason":"x","admin":"y"}"#).is_err());
        let order: BanOrder = serde_json::from_str(r#"{"reason":"spam"}"#).unwrap();
        assert_eq!(
            serde_json::to_string(&order).unwrap(),
            r#"{"reason":"spam"}"#
        );
    }
}
