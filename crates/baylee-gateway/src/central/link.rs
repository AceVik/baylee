//! Linking the gateway to Baylee Central with a token its owner made on the
//! website (baylee-central `docs/gateway-registry.md` §"Gateway tokens";
//! `docs/central-login.md` §"Linking a gateway with a token"). An admin
//! pastes it into the console; the gateway asks Central what the token is
//! (`GatewayStatus`), keeps it in its database (it presents the token, so it
//! cannot keep a digest), and uses it at once for its players' sign-ins and
//! its directory heartbeat, for what the token's scopes allow. The operator's
//! file (`BAYLEE_CENTRAL_CONFIG_FILE`) takes precedence: the console shows a
//! link from it and cannot change it. The token never reaches a log or an
//! answer; the console sees its last four characters.

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, put};
use baylee_account::gateway::{GatewayMetadata, GatewayState};
use baylee_account::{Token, owner, wire};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use zeroize::Zeroizing;

use super::Bridge;
use crate::admin::{Actor, Admin, Refusal, iso};
use crate::{Shared, err};

/// The console's routes for the link.
pub(crate) fn console_routes() -> Router<Admin> {
    Router::new()
        .route("/admin/central", get(current))
        .route("/admin/central/token", put(paste).delete(remove))
}

fn db_failed(e: &sea_orm::DbErr) -> Refusal {
    tracing::error!("admin console: central link: {e}");
    err(
        StatusCode::SERVICE_UNAVAILABLE,
        "the gateway's database is unavailable",
    )
}

/// The heartbeat's directory entry: this gateway's own name
/// (`BAYLEE_GATEWAY_NAME`) where Central's rules take it, else the name it was
/// registered under; its version; the website Central holds, on its origin.
fn entry(state: &Shared, origin: &str, registered: &GatewayMetadata) -> Option<GatewayMetadata> {
    let mut metadata = GatewayMetadata {
        name: state
            .display_name
            .clone()
            .unwrap_or_else(|| registered.name.clone()),
        version: env!("CARGO_PKG_VERSION").into(),
        website: registered.website.clone(),
    };
    if metadata.validate(origin).is_err() {
        metadata.name.clone_from(&registered.name);
    }
    metadata.validate(origin).ok().map(|()| metadata)
}

/// What Central says about `token`, through a bridge of it.
async fn status(bridge: &Arc<Bridge>) -> Result<wire::Response, StatusCode> {
    bridge.exchange(wire::Request::GatewayStatus).await
}

/// Read the pasted token from the database at start, unless the operator's
/// file set the link. A token stored before is used as it is: Central checks
/// it on every request, so a revoked or expired one fails there, not here.
pub(crate) async fn restore(state: &Shared) {
    if state.central.fixed_by_file() {
        return;
    }
    let stored = match baylee_db::central::link(&state.db).await {
        Ok(Some(stored)) => stored,
        Ok(None) => return,
        Err(e) => {
            tracing::error!("central link: the stored token could not be read: {e}");
            return;
        }
    };
    let token = Token::from_secret_bytes(Zeroizing::new(stored.token));
    // Central is asked for the entry once; while it is away the gateway
    // beats with its own name and the origin's root.
    let probe = Bridge::from_token(
        state.central.endpoint.clone(),
        &token,
        placeholder(&stored.origin),
    );
    let metadata = match status(&probe).await {
        Ok(wire::Response::GatewayStatus {
            origin, metadata, ..
        }) if origin == stored.origin => entry(state, &origin, &metadata),
        _ => entry(state, &stored.origin, &placeholder(&stored.origin)),
    };
    let Some(metadata) = metadata else {
        tracing::error!("central link: no directory entry fits the stored origin; not linked");
        return;
    };
    state.central.set(Some(Bridge::from_token(
        state.central.endpoint.clone(),
        &token,
        metadata,
    )));
    tracing::info!(gateway = %uuid::Uuid::from_bytes(stored.gateway), "linked to Baylee Central by its stored token");
}

/// The entry before Central named one: the origin's root, version `0`.
fn placeholder(origin: &str) -> GatewayMetadata {
    GatewayMetadata {
        name: "Baylee gateway".into(),
        version: "0".into(),
        website: format!("{origin}/"),
    }
}

/// `GET /admin/central` and the answer of a change.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleCentral {
    /// @description Where the link comes from: the operator's file
    /// (`BAYLEE_CENTRAL_CONFIG_FILE`, which the console cannot change), a
    /// token pasted here, or none.
    /// @openapi enum: [file, token, none]
    source: &'static str,
    /// @description The pasted token, never more of it than its last four
    /// characters; `null` unless `source` is `token`.
    token: Option<ConsoleToken>,
    /// @description What Baylee Central says about the token now; `null`
    /// unless `source` is `token` and Central answered.
    central: Option<ConsoleStatus>,
    /// @description How asking Central went: `ok`, `rejected` (it does not
    /// know the token, or it was revoked or has expired: make a new one on
    /// the website) or `unavailable`; `null` when nothing was asked.
    /// @openapi enum: [ok, rejected, unavailable]
    check: Option<&'static str>,
}

/// The pasted token, as the console shows it.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleToken {
    /// @description Its last four characters.
    last4: String,
    /// @description When it was pasted.
    set_at: String,
    /// @description Who pasted it: the console's admin name, or a Central
    /// account's id.
    set_by: String,
}

/// What Baylee Central says about the token.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleStatus {
    /// @description The gateway's id at Central.
    gateway: String,
    /// @description Its origin, the audience of its players' tickets.
    origin: String,
    /// @description The name Central lists it under.
    name: String,
    /// @description Whether an administrator of Central approved it; only an
    /// approved gateway is listed and signs players in.
    /// @openapi enum: [pending, approved, disabled]
    state: &'static str,
    /// @description The token signs this gateway's players in.
    sso: bool,
    /// @description The token keeps this gateway's directory entry.
    listing: bool,
    /// @description When the token stops working.
    expires_at: String,
}

fn state_word(state: GatewayState) -> &'static str {
    match state {
        GatewayState::Pending => "pending",
        GatewayState::Approved => "approved",
        GatewayState::Disabled => "disabled",
    }
}

fn status_of(
    response: Result<wire::Response, StatusCode>,
) -> (Option<ConsoleStatus>, &'static str) {
    match response {
        Ok(wire::Response::GatewayStatus {
            gateway,
            origin,
            metadata,
            state,
            scopes,
            expires_at,
        }) => (
            Some(ConsoleStatus {
                gateway: uuid::Uuid::from_bytes(gateway).to_string(),
                origin,
                name: metadata.name,
                state: state_word(state),
                sso: scopes.sso,
                listing: scopes.listing,
                expires_at: OffsetDateTime::from_unix_timestamp(expires_at)
                    .map_or_else(|_| String::new(), iso),
            }),
            "ok",
        ),
        Err(StatusCode::UNAUTHORIZED) => (None, "rejected"),
        _ => (None, "unavailable"),
    }
}

async fn describe(state: &Shared) -> Result<ConsoleCentral, Refusal> {
    if state.central.fixed_by_file() {
        return Ok(ConsoleCentral {
            source: "file",
            token: None,
            central: None,
            check: None,
        });
    }
    let Some(stored) = baylee_db::central::link(&state.db)
        .await
        .map_err(|e| db_failed(&e))?
    else {
        return Ok(ConsoleCentral {
            source: "none",
            token: None,
            central: None,
            check: None,
        });
    };
    let token = Token::from_secret_bytes(Zeroizing::new(stored.token));
    let text = owner::token_text(&token);
    let (central, check) = match state.central.get() {
        Some(bridge) => status_of(status(&bridge).await),
        None => (None, "unavailable"),
    };
    Ok(ConsoleCentral {
        source: "token",
        token: Some(ConsoleToken {
            last4: text[text.len() - 4..].to_owned(),
            set_at: OffsetDateTime::from_unix_timestamp(stored.set_at)
                .map_or_else(|_| String::new(), iso),
            set_by: stored.set_by,
        }),
        central,
        check: Some(check),
    })
}

/// `GET /admin/central`: where the link comes from and, for a pasted token,
/// what Baylee Central says about it now.
///
/// @route GET /admin/central
/// @scope admin
/// @summary The gateway's link to Baylee Central
/// @description Where the link comes from (the operator's file, a pasted
/// token, or none) and, for a pasted token, what Baylee Central says about
/// it now: the gateway's origin and name, whether it is approved, what the
/// token allows and until when. Never the token, only its last four
/// characters.
/// @tag Admin
/// @operation-id `getCentralLink`
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 200: `$ConsoleCentral` "The link"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn current(State(console): State<Admin>) -> Result<Json<ConsoleCentral>, Refusal> {
    Ok(Json(describe(&console.app).await?))
}

/// What `PUT /admin/central/token` takes.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasteBody {
    /// @description The token as the website showed it: `bgt_` and 64
    /// lowercase hex digits; surrounding spaces are ignored.
    token: String,
}

/// `PUT /admin/central/token {token}`: links the gateway with a token made on
/// the website, once Baylee Central knows it. `200` with what `GET
/// /admin/central` answers.
///
/// @route PUT /admin/central/token
/// @scope admin
/// @summary Link the gateway to Baylee Central with a token
/// @description Asks Baylee Central what the token is, keeps it in the
/// gateway's database, and uses it at once: for its players' sign-ins
/// through Central and its entry in the public gateway list, as far as the
/// token's scopes allow and once an administrator of Central approved the
/// gateway. Replaces a token pasted before. Not while the operator's file
/// sets the link.
/// @tag Admin
/// @operation-id `pasteCentralToken`
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line; ignored when the bearer is a Central admin lease, whose account acts"
/// @body `$PasteBody`
/// @return 200: `$ConsoleCentral` "Linked, as `GET /admin/central` answers"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, or not a token: `bgt_` and 64 lowercase hex digits"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 409: `$ErrorBody` "The operator's file (`BAYLEE_CENTRAL_CONFIG_FILE`) sets the link"
/// @return 413: "The body is over 16 KiB"
/// @return 422: `$ErrorBody` "Baylee Central does not know the token, or it was revoked or has expired"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable"
async fn paste(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    body: Bytes,
) -> Result<Response, Refusal> {
    let state = &console.app;
    if state.central.fixed_by_file() {
        return Err(err(
            StatusCode::CONFLICT,
            "the operator's file sets this gateway's link to Baylee Central",
        ));
    }
    let body: PasteBody = serde_json::from_slice(&body)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "send {\"token\": \"bgt_…\"}"))?;
    let token = owner::parse_token_text(&body.token).map_err(|_| {
        err(
            StatusCode::BAD_REQUEST,
            "not a gateway token: bgt_ and 64 lowercase hex digits",
        )
    })?;
    drop(body);
    let probe = Bridge::from_token(
        state.central.endpoint.clone(),
        &token,
        placeholder("https://unknown.invalid"),
    );
    let (gateway, origin, metadata) = match status(&probe).await {
        Ok(wire::Response::GatewayStatus {
            gateway,
            origin,
            metadata,
            ..
        }) => (gateway, origin, metadata),
        Err(StatusCode::UNAUTHORIZED) => {
            return Err(err(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Baylee Central does not know this token, or it was revoked or has expired",
            ));
        }
        _ => {
            return Err(err(
                StatusCode::SERVICE_UNAVAILABLE,
                "Baylee Central could not be asked about the token",
            ));
        }
    };
    let entry = entry(state, &origin, &metadata).ok_or_else(|| {
        err(
            StatusCode::SERVICE_UNAVAILABLE,
            "Baylee Central's entry for this gateway does not fit its origin",
        )
    })?;
    baylee_db::central::set_link(
        &state.db,
        &token.to_secret_bytes(),
        gateway,
        &origin,
        &actor,
    )
    .await
    .map_err(|e| db_failed(&e))?;
    state.central.set(Some(Bridge::from_token(
        state.central.endpoint.clone(),
        &token,
        entry,
    )));
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "central.link",
        gateway = %uuid::Uuid::from_bytes(gateway),
        "admin console: linked to Baylee Central by a token"
    );
    // a pasted token links the gateway: its entry goes online and its terms
    // go to Central now, not at the next tick
    let app = state.clone();
    tokio::spawn(async move {
        if let Some(bridge) = app.central.get()
            && let Some(metadata) = bridge.metadata.clone()
        {
            let _ = bridge
                .exchange(wire::Request::GatewayHeartbeat { metadata })
                .await;
        }
        super::publish_terms(&app).await;
    });
    Ok(Json(describe(state).await?).into_response())
}

/// `DELETE /admin/central/token`: unlinks the gateway. `204`; its players
/// signed in through Central are signed out at their next check.
///
/// @route DELETE /admin/central/token
/// @scope admin
/// @summary Remove the pasted token
/// @description Forgets the token and unlinks the gateway at once: it
/// signs nobody in through Baylee Central, players signed in that way are
/// signed out at their next check, and its directory entry goes offline. The
/// token itself stays valid at Central until it is revoked on the website.
/// @tag Admin
/// @operation-id `removeCentralToken`
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line; ignored when the bearer is a Central admin lease, whose account acts"
/// @return 204: "Unlinked"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "No token is pasted"
/// @return 409: `$ErrorBody` "The operator's file (`BAYLEE_CENTRAL_CONFIG_FILE`) sets the link"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn remove(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
) -> Result<StatusCode, Refusal> {
    let state = &console.app;
    if state.central.fixed_by_file() {
        return Err(err(
            StatusCode::CONFLICT,
            "the operator's file sets this gateway's link to Baylee Central",
        ));
    }
    if !baylee_db::central::clear_link(&state.db)
        .await
        .map_err(|e| db_failed(&e))?
    {
        return Err(err(StatusCode::NOT_FOUND, "no token is pasted"));
    }
    state.central.set(None);
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "central.unlink",
        "admin console: unlinked from Baylee Central"
    );
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn central_s_answer_is_read_without_the_token() {
        let (status, check) = status_of(Err(StatusCode::UNAUTHORIZED));
        assert!(status.is_none());
        assert_eq!(check, "rejected");
        assert_eq!(
            status_of(Err(StatusCode::SERVICE_UNAVAILABLE)).1,
            "unavailable"
        );
        let (status, check) = status_of(Ok(wire::Response::GatewayStatus {
            gateway: [1; 16],
            origin: "https://gateway.example".into(),
            metadata: placeholder("https://gateway.example"),
            state: GatewayState::Pending,
            scopes: owner::GatewayScopes {
                sso: true,
                listing: false,
            },
            expires_at: 1_800_000_000,
        }));
        let status = status.unwrap();
        assert_eq!(check, "ok");
        assert_eq!(status.state, "pending");
        assert!(status.sso && !status.listing);
        assert_eq!(status.expires_at, "2027-01-15T08:00:00Z");
    }

    #[test]
    fn the_placeholder_entry_fits_its_origin() {
        placeholder("https://gateway.example")
            .validate("https://gateway.example")
            .unwrap();
    }
}
