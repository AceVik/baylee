use super::{Bridge, Shared};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::post,
};
use baylee_account::{
    migration::Capability,
    wire::{Identity, Request, Response, TicketPurpose},
};
use std::{sync::Arc, time::Duration};

fn bridge(state: &Shared) -> Result<Arc<Bridge>, StatusCode> {
    state.central.get().ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

async fn exclusive(state: &Shared) -> Result<tokio::sync::RwLockWriteGuard<'_, ()>, StatusCode> {
    tokio::time::timeout(Duration::from_secs(5), state.central_barrier.write())
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
}

async fn allowed(state: &Shared, account_id: &str) -> Result<crate::store::Account, StatusCode> {
    let account = crate::store::account(&state.db, account_id)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        .filter(|a| !a.guest)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if account.ban_in_force(crate::auth::now_secs()).is_some() || account.must_change_password {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(account)
}

async fn redeem(
    state: &Shared,
    capability: Capability,
    purpose: TicketPurpose,
) -> Result<
    (
        Identity,
        Option<String>,
        Option<baylee_account::Token>,
        Option<i64>,
    ),
    StatusCode,
> {
    match bridge(state)?
        .exchange(Request::RedeemGateway {
            ticket: capability.ticket,
            purpose,
        })
        .await?
    {
        Response::GatewayIdentity {
            identity,
            legacy_id,
            lease,
            expires_at,
        } => Ok((identity, legacy_id, lease, expires_at)),
        _ => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// `POST /auth/central/login`: a Baylee Central account admitted here.
///
/// @route POST /auth/central/login
/// @scope public
/// @operation-id `central_login`
/// @summary Sign in with Baylee Central
/// @description Trades an `admit` ticket from Baylee Central for a session
/// at this gateway, for the account Central names (a gateway that is not
/// the migration gateway makes a local projection of it the first time,
/// only while its registration is open). Answered as a sign-in is, and the
/// session's every request is checked with Central. Every refusal is a bare
/// status.
/// @tag Central
/// @security none
/// @body `$CentralCapability`
/// @return 200: `$CentralSession` "The session"
/// @return 401: "The ticket is not good, the account is not linked here, it is a guest, or Central no longer admits it"
/// @return 403: "This gateway takes no Central account (its mode is `guests`), the account is banned or must choose a new password first, or Central refused"
/// @return 409: "Central answered a conflict"
/// @return 413: String text/plain "The body is over 8 KiB"
/// @return 429: "Central is busy"
/// @return 503: "No Baylee Central is configured, the database or Central is unavailable, or the account is new here and this gateway's registration is not open"
async fn login(
    State(state): State<Shared>,
    Json(capability): Json<Capability>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // The mode in force decides, before Central is asked anything
    // (`authmode.rs`); the console's own sign-in below follows none.
    if !state.auth_mode.current().central() {
        return Err(StatusCode::FORBIDDEN);
    }
    login_for(&state, capability, TicketPurpose::Admit).await
}

/// `POST /auth/central/admin`: a gateway admin, by Baylee Central.
///
/// @route POST /auth/central/admin
/// @scope public
/// @operation-id `central_admin_login`
/// @summary Sign in to the admin console with Baylee Central
/// @description Trades an `admin` ticket from Baylee Central for a bearer
/// that works on the admin listener only, where each request checks the
/// Central session and the account's admin membership. Not on the migration
/// gateway. Every refusal is a bare status.
/// @tag Central
/// @security none
/// @body `$CentralCapability`
/// @return 200: `$CentralAdminSession` "The admin bearer"
/// @return 401: "The ticket is not good, or the account is a guest"
/// @return 403: "This is the migration gateway, the account is banned or must choose a new password first, or Central refused"
/// @return 409: "Central answered a conflict"
/// @return 413: String text/plain "The body is over 8 KiB"
/// @return 429: "Central is busy"
/// @return 503: "No Baylee Central is configured, or the database or Central is unavailable"
async fn admin_login(
    State(state): State<Shared>,
    Json(capability): Json<Capability>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if bridge(&state)?.legacy {
        return Err(StatusCode::FORBIDDEN);
    }
    login_for(&state, capability, TicketPurpose::Admin).await
}

/// Takes the name and tag the account has at Central onto its projection
/// here (`GatewayProfile`), at each sign-in, so a name chosen or changed on
/// the website arrives with the next one. A name this gateway's rule
/// refuses is not taken. Any refusal, Central being away included, leaves
/// the projection as it is and the sign-in goes on; answers whether the
/// projection was written.
async fn take_profile(
    state: &Shared,
    lease: &baylee_account::Token,
    purpose: TicketPurpose,
    central: [u8; 16],
    account_id: &str,
) -> bool {
    let Ok(bridge) = bridge(state) else {
        return false;
    };
    let Ok(Response::GatewayProfile {
        identity,
        display_name,
        tag,
    }) = bridge
        .exchange(Request::GatewayProfile {
            lease: baylee_account::Token::from_secret_bytes(lease.to_secret_bytes()),
            purpose,
        })
        .await
    else {
        return false;
    };
    if identity.account != central {
        return false;
    }
    let display_name = display_name.filter(|name| crate::auth::valid_display_name(name));
    let tag = tag
        .and_then(|tag| i32::try_from(tag).ok())
        .filter(|tag| *tag > 0);
    baylee_db::central::take_profile(&state.db, account_id, display_name.as_deref(), tag)
        .await
        .unwrap_or(false)
}

async fn login_for(
    state: &Shared,
    capability: Capability,
    purpose: TicketPurpose,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (identity, legacy, lease, expires_at) = redeem(state, capability, purpose).await?;
    let (Some(lease), Some(expires_at)) = (lease, expires_at) else {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };
    let _guard = exclusive(state).await?;
    let admin = matches!(purpose, TicketPurpose::Admin);
    let account_id = match (bridge(state)?.legacy, legacy) {
        // The official gateway: a moved account is the legacy account it
        // moved from, and nothing else. Central names the legacy account to
        // the bridge only once its move is closed (baylee-central
        // `docs/native-login.md` §"Legacy login"); the first admission fixes
        // the link here, ends the account's password and its password
        // sessions, and asks its sockets to sign in again.
        (true, Some(legacy)) => {
            match baylee_db::central::adopt(&state.db, &legacy, identity.account).await {
                Ok(true) => {
                    let _ = state.departed.send(legacy.clone());
                }
                Ok(false) => {}
                Err(sea_orm::DbErr::Custom(_)) => return Err(StatusCode::UNAUTHORIZED),
                Err(_) => return Err(StatusCode::SERVICE_UNAVAILABLE),
            }
            legacy
        }
        // An account registered at Central has no legacy account: the
        // official gateway makes it a projection, as a registered gateway
        // does. Central's beta key stood in for this gateway's at sign-up,
        // so only `BAYLEE_REGISTRATION=off` closes the door (baylee-central
        // `docs/web-accounts.md`).
        (true, None) => baylee_db::central::projection(
            &state.db,
            identity.account,
            admin || state.registration != crate::invite::Registration::Off,
        )
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        (false, _) => baylee_db::central::projection(
            &state.db,
            identity.account,
            admin || state.registration == crate::invite::Registration::Open,
        )
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    let mut account = allowed(state, &account_id).await?;
    if take_profile(state, &lease, purpose, identity.account, &account_id).await {
        account = allowed(state, &account_id).await?;
    }
    let token = crate::auth::new_token();
    baylee_db::central::issue_session(
        &state.db,
        &account_id,
        crate::auth::token_digest(&token),
        *lease.to_secret_bytes(),
        expires_at,
        purpose.as_str(),
    )
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let mut answer = serde_json::json!({
        "token": token, "expires_at": expires_at, "username": account.username,
        "display_name": account.display_name,
    });
    // Whether the account accepted the terms in force is Central's to say
    // (`docs/protocol.md` §"Terms of use (WG-1)"); an admin session is
    // checked only for its lease.
    if matches!(purpose, TicketPurpose::Admit) {
        let bridge = bridge(state)?;
        let standing = crate::central::authorize(
            state,
            &bridge,
            *lease.to_secret_bytes(),
            identity.account,
            expires_at,
        )
        .await?
        .ok_or(StatusCode::UNAUTHORIZED)?;
        crate::routes::mark_standing(state, standing, &mut answer);
    }
    Ok(Json(answer))
}

/// The Central routes, outside the migration barrier (each takes it itself)
/// and with a body limit of their own.
///
/// The bodies are `baylee_account::migration`'s types, which the API
/// document is not read from, so their schemas are spelled here. A `Token`
/// is 32 bytes as a JSON array of numbers (`[u8; 32]`).
///
/// @openapi document
/// ```yaml
/// components:
///   schemas:
///     CentralBytes32:
///       type: array
///       description: '32 bytes, as a JSON array of 32 numbers from 0 to 255.'
///       items: { type: integer, minimum: 0, maximum: 255 }
///       minItems: 32
///       maxItems: 32
///     CentralCapability:
///       type: object
///       description: 'A one-use ticket Baylee Central issued for this gateway and this route''s purpose, good for 30 seconds.'
///       properties:
///         ticket:
///           $ref: '#/components/schemas/CentralBytes32'
///       required: [ticket]
///       additionalProperties: false
///     CentralAdminSession:
///       type: object
///       description: 'A bearer for the admin listener.'
///       properties:
///         token:
///           type: string
///           description: 'The bearer: 64 lowercase hex characters.'
///         expires_at:
///           type: integer
///           format: int64
///           description: 'When it ends, Unix seconds: the end of its Central lease.'
///         username:
///           type: [string, 'null']
///           description: 'The account''s username here, `null` for an account only Central knows.'
///         display_name:
///           type: string
///           description: 'The name other players see.'
///       required: [token, expires_at, username, display_name]
///     CentralSession:
///       type: object
///       description: 'A session at this gateway, answered as a sign-in is.'
///       properties:
///         token:
///           type: string
///           description: 'The session''s bearer token: 64 lowercase hex characters.'
///         expires_at:
///           type: integer
///           format: int64
///           description: 'When it ends, Unix seconds: the end of its Central lease.'
///         username:
///           type: [string, 'null']
///           description: 'The account''s username here, `null` for an account only Central knows.'
///         display_name:
///           type: string
///           description: 'The name other players see.'
///         terms_stale:
///           type: boolean
///           description: 'Whether the account has yet to accept this gateway''s terms in force (at Central); only on a gateway with terms.'
///         central_terms_stale:
///           type: boolean
///           const: true
///           description: 'Only there when the account has yet to accept Baylee Central''s own terms, which come first.'
///       required: [token, expires_at, username, display_name]
/// ```
pub(crate) fn router() -> Router<Shared> {
    Router::new()
        .route("/auth/central/login", post(login))
        .route("/auth/central/admin", post(admin_login))
        .layer(DefaultBodyLimit::max(8192))
}
