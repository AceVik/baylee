//! The admin pages' acts on an account or a game (beta.8, `docs/feedback.md`
//! §"The admin console"), passed on to the gateway's console as the reads
//! are ([`crate::console`]), for a signed-in admin with the UI's CSRF proof:
//!
//! - `POST /ui/api/admin/accounts/{id}/password`: a registered account's
//!   password reset; the answer carries the new one, the one time it is
//!   shown, and nothing here writes it down.
//! - `PUT /ui/api/admin/accounts/{id}/ban` `{reason, until?}`: a ban;
//!   `DELETE` the same path lifts it.
//! - `DELETE /ui/api/admin/accounts/{id}`: the account deleted, as its
//!   player's own deletion does.
//! - `POST /ui/api/admin/games/{id}/close` `{reason}`: a running game
//!   ended through its engine.
//!
//! Each is written into `feedback_audit` with the admin and the account or
//! game, never the password, nor a reason, which may name a person.

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
        .route("/ui/api/admin/accounts/{id}/password", post(reset_password))
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

async fn reset_password(
    State(shared): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Response, Refusal> {
    same_origin(&headers)?;
    let (admin, gateway) = signed_in(&shared, &headers).await?;
    let id = account_id(&id)?;
    let done = ask(
        &gateway,
        "POST",
        format!("/admin/accounts/{id}/password"),
        &admin,
        Some(b"{}".to_vec()),
    )
    .await?;
    if done.status == 200 {
        noted(
            &shared,
            &admin,
            "gateway.account.password.reset",
            &format!("account {id}"),
        )
        .await;
    }
    Ok(answer(done))
}

/// What `PUT /ui/api/admin/accounts/{id}/ban` takes, and all it passes on.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BanOrder {
    reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    until: Option<String>,
}

/// The audit's words for a ban: the account and its end, never the reason.
fn ban_detail(id: Uuid, until: Option<&str>) -> String {
    format!("account {id}, until {}", until.unwrap_or("never"))
}

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
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CloseOrder {
    reason: String,
}

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
