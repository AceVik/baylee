use super::{Bridge, Shared, trust::Refused};
use crate::sessions::{self, Ended, Signed};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{ConnectInfo, DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
    routing::post,
};
use baylee_protocol::refusal::Refusal;
use central_crypto::jwt::{GatewayClaims, Kind, Purpose};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use zeroize::Zeroizing;

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

/// The key a Central session is known by here: its `sid`, which stays the
/// same across its tokens and its bearers.
pub(crate) fn key_of(sid: &str) -> sessions::Key {
    format!("central:{sid}").into_bytes()
}

/// What a sign-in or a refresh sends.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SignIn {
    token: String,
}

/// What a client from before gateway tokens sends.
#[derive(Deserialize)]
struct OldSignIn {
    #[serde(rename = "ticket")]
    _ticket: serde::de::IgnoredAny,
}

/// The gateway token in a body. A client too old to have one hears that
/// (`docs/protocol.md` §"Refusals"); anything else is a bad request.
fn read_token(body: &[u8]) -> Result<Zeroizing<String>, Box<Response>> {
    if let Ok(sign_in) = serde_json::from_slice::<SignIn>(body) {
        return Ok(Zeroizing::new(sign_in.token));
    }
    if serde_json::from_slice::<OldSignIn>(body).is_ok() {
        let refusal = json!({ "refusal": Refusal::update_required() });
        return Err(Box::new(
            (StatusCode::UPGRADE_REQUIRED, Json(refusal)).into_response(),
        ));
    }
    Err(Box::new(StatusCode::BAD_REQUEST.into_response()))
}

/// A gateway token checked here, with no word from Central unless it names a
/// key not held, which may ask for the keys once.
async fn checked(bridge: &Arc<Bridge>, token: &str) -> Result<GatewayClaims, StatusCode> {
    let first = bridge.trust.gateway_token(token, sessions::now());
    let claims = if matches!(first, Err(Refused::UnknownKey)) {
        bridge.refresh_keys(true).await;
        bridge.trust.gateway_token(token, sessions::now())
    } else {
        first
    };
    claims.map_err(|why| match why {
        // Not a refusal of this token: nothing can be said of any yet.
        Refused::NotReady => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::UNAUTHORIZED,
    })
}

/// Takes the name and tag the account has at Central onto its projection
/// here, at each sign-in, so a name chosen or changed on the website arrives
/// with the next one. A name this gateway's rule refuses is not taken.
/// Answers whether the projection was written.
async fn take_profile(state: &Shared, claims: &GatewayClaims, account_id: &str) -> bool {
    let display_name = claims
        .name
        .as_deref()
        .filter(|name| crate::auth::valid_display_name(name));
    let tag = claims
        .tag
        .and_then(|tag| i32::try_from(tag).ok())
        .filter(|tag| *tag > 0);
    baylee_db::central::take_profile(&state.db, account_id, display_name, tag)
        .await
        .unwrap_or(false)
}

/// The session of a checked token, opened unless it was revoked meanwhile.
///
/// Asked after opening, and that order is the whole point: a revocation writes
/// its `sid` down and then ends the session ([`ended`]), and a sign-in opens
/// the session and then reads that list. Whichever comes first, one sees the
/// other, so a revocation that lands during the waits of a sign-in cannot leave
/// a session standing.
fn opened(
    book: &sessions::Book,
    trust: &super::trust::Trust,
    claims: &GatewayClaims,
    account: &str,
    now: i64,
    signed: Signed,
) -> Result<Arc<sessions::Live>, StatusCode> {
    opened_around(book, trust, claims, account, now, signed, || {})
}

/// [`opened`], with `between` run at the one point that matters, so that a test
/// can put a revocation exactly there instead of hoping two threads meet.
fn opened_around(
    book: &sessions::Book,
    trust: &super::trust::Trust,
    claims: &GatewayClaims,
    account: &str,
    now: i64,
    signed: Signed,
    between: impl FnOnce(),
) -> Result<Arc<sessions::Live>, StatusCode> {
    let key = key_of(&claims.sid);
    let live = book
        .open(key.clone(), account, claims.exp, now, signed)
        .map_err(|sessions::Full| StatusCode::SERVICE_UNAVAILABLE)?;
    if let Some(signed) = live.signed() {
        signed.set_terms(claims.terms.clone());
    }
    between();
    if trust.revoked(&claims.sid) {
        book.end(&key, Ended::Revoked);
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(live)
}

/// A sign-in from a gateway token: the account, its session in the book, and a
/// bearer for it.
async fn sign_in(state: &Shared, token: &str, purpose: Purpose) -> Result<Json<Value>, StatusCode> {
    let bridge = bridge(state)?;
    let claims = checked(&bridge, token).await?;
    if claims.purpose != purpose {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let central = claims.account().ok_or(StatusCode::UNAUTHORIZED)?;
    // The barrier drains the requests of a legacy account while its link is
    // fixed, and that is the only sign-in with anything to wait for: a new
    // projection has no requests, and one refused leaves nothing. Taking it
    // for those would let any Central account refuse a sign-in and stall
    // every route behind any slow request, as often as it liked.
    let adopting = match claims.legacy_id.as_ref().filter(|_| bridge.legacy) {
        Some(legacy) => {
            baylee_db::central::linked_account(&state.db, central)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
                .as_ref()
                != Some(legacy)
        }
        None => false,
    };
    let _guard = if adopting {
        Some(exclusive(state).await?)
    } else {
        None
    };
    let admin = purpose == Purpose::Admin;
    let account_id = match (bridge.legacy, claims.legacy_id.clone()) {
        // The official gateway: a moved account is the legacy account it
        // moved from, and nothing else. Central names the legacy account only
        // once its move is closed (baylee-central `docs/native-login.md`
        // §"Legacy login"); the first admission fixes the link here, ends the
        // account's password and its password sessions, and asks its sockets
        // to sign in again.
        (true, Some(legacy)) => {
            match baylee_db::central::adopt(&state.db, &legacy, central).await {
                Ok(true) => {
                    state.sessions.end_account(&legacy, Ended::SignedOut);
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
            central,
            admin || state.registration != crate::invite::Registration::Off,
        )
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        (false, _) => baylee_db::central::projection(
            &state.db,
            central,
            admin || state.registration == crate::invite::Registration::Open,
        )
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    };
    let mut account = allowed(state, &account_id).await?;
    if take_profile(state, &claims, &account_id).await {
        account = allowed(state, &account_id).await?;
    }
    let now = sessions::now();
    let signed = Signed::new(central, admin, claims.terms.clone());
    let live = opened(
        &state.sessions,
        &bridge.trust,
        &claims,
        &account_id,
        now,
        signed,
    )?;
    let issued = state
        .sessions
        .issue(&live, now)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let mut answer = json!({
        "token": issued.token,
        "expires_at": issued.expires_at,
        "renew_in": issued.renew_in,
        "session_expires_at": live.expires_at(),
        "username": account.username,
        "display_name": account.display_name,
    });
    // Whether the account accepted this gateway's terms is in its token;
    // Central's own are a condition of the token. An admin session is
    // checked for neither (`docs/protocol.md` §"Terms of use (WG-1)").
    if !admin {
        crate::routes::mark_standing(state, super::standing(state, &live), &mut answer);
    }
    Ok(Json(answer))
}

/// Whether this address may have a token checked now. A signature is checked
/// in this process, for anyone who sends one, so a flood is stopped here.
fn within_budget(state: &Shared, addr: SocketAddr, headers: &HeaderMap) -> bool {
    let ip = crate::rate_limit_ip(&state.trusted_proxies, addr.ip(), headers);
    state.token_limiter.allow(&ip)
}

/// Whether this address may have a pushed message checked now. Its own
/// budget and nobody else's, so that a stranger at this door costs Central's
/// own pushes nothing.
fn within_push_budget(state: &Shared, addr: SocketAddr, headers: &HeaderMap) -> bool {
    let ip = crate::rate_limit_ip(&state.trusted_proxies, addr.ip(), headers);
    state.invalidate_limiter.allow(&ip)
}

fn answered(result: Result<Json<Value>, StatusCode>) -> Response {
    result.map_or_else(IntoResponse::into_response, IntoResponse::into_response)
}

/// `POST /auth/central/login`: a Baylee Central account admitted here.
///
/// @route POST /auth/central/login
/// @scope public
/// @operation-id `central_login`
/// @summary Sign in with Baylee Central
/// @description Trades a gateway token from Baylee Central for a session at
/// this gateway, for the account it names (a gateway that is not the migration
/// gateway makes a local projection of it the first time, only while its
/// registration is open). The token is checked here, with no word from
/// Central. Answered as a sign-in is. Every refusal is a bare status, but for
/// `426`.
/// @tag Central
/// @security none
/// @body `$CentralSignIn`
/// @return 200: `$CentralSession` "The session"
/// @return 400: "The body is no sign-in"
/// @return 401: "The token is not good here, its session was revoked, the account is not linked here or it is a guest"
/// @return 403: "This gateway takes no Central account (its mode is `guests`), or the account is banned or must choose a new password first"
/// @return 413: String text/plain "The body is over 8 KiB"
/// @return 426: `$RefusalBody` "A client from before gateway tokens: `update_required`"
/// @return 429: "Too many sign-ins from this address"
/// @return 503: "No Baylee Central is configured, this gateway has not caught up with Central since it started, the database is unavailable, or the account is new here and this gateway's registration is not open"
async fn login(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !within_budget(&state, addr, &headers) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    // The mode in force decides, before anything is read (`authmode.rs`);
    // the console's own sign-in below follows none.
    if !state.auth_mode.current().central() {
        return StatusCode::FORBIDDEN.into_response();
    }
    match read_token(&body) {
        Ok(token) => answered(sign_in(&state, &token, Purpose::Admit).await),
        Err(refused) => *refused,
    }
}

/// `POST /auth/central/admin`: a gateway admin, by Baylee Central.
///
/// @route POST /auth/central/admin
/// @scope public
/// @operation-id `central_admin_login`
/// @summary Sign in to the admin console with Baylee Central
/// @description Trades an `admin` gateway token from Baylee Central for a
/// bearer that works on the admin listener only. It lasts as long as the token
/// and is not renewed; Central revokes it when the account stops being an
/// admin here. Not on the migration gateway. Every refusal is a bare status,
/// but for `426`.
/// @tag Central
/// @security none
/// @body `$CentralSignIn`
/// @return 200: `$CentralAdminSession` "The admin bearer"
/// @return 400: "The body is no sign-in"
/// @return 401: "The token is not good here, its session was revoked, or the account is a guest"
/// @return 403: "This is the migration gateway, or the account is banned or must choose a new password first"
/// @return 413: String text/plain "The body is over 8 KiB"
/// @return 426: `$RefusalBody` "A client from before gateway tokens: `update_required`"
/// @return 429: "Too many sign-ins from this address"
/// @return 503: "No Baylee Central is configured, this gateway has not caught up with Central since it started, or the database is unavailable"
async fn admin_login(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !within_budget(&state, addr, &headers) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    match bridge(&state) {
        Ok(bridge) if bridge.legacy => return StatusCode::FORBIDDEN.into_response(),
        Ok(_) => {}
        Err(status) => return status.into_response(),
    }
    match read_token(&body) {
        Ok(token) => answered(sign_in(&state, &token, Purpose::Admin).await),
        Err(refused) => *refused,
    }
}

/// A later gateway token of a session this gateway holds.
async fn refreshed(state: &Shared, token: &str) -> Result<Json<Value>, StatusCode> {
    let bridge = bridge(state)?;
    let claims = checked(&bridge, token).await?;
    let now = sessions::now();
    let live = state
        .sessions
        .get(&key_of(&claims.sid), now)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let signed = live.signed().ok_or(StatusCode::UNAUTHORIZED)?;
    let admin = claims.purpose == Purpose::Admin;
    if claims.account() != Some(signed.central) || admin != signed.admin {
        return Err(StatusCode::UNAUTHORIZED);
    }
    live.extend(claims.exp);
    signed.set_terms(claims.terms);
    let issued = state
        .sessions
        .issue(&live, now)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let mut answer = json!({
        "token": issued.token,
        "expires_at": issued.expires_at,
        "renew_in": issued.renew_in,
        "session_expires_at": live.expires_at(),
    });
    if !admin {
        crate::routes::mark_standing(state, super::standing(state, &live), &mut answer);
    }
    Ok(Json(answer))
}

/// `POST /auth/central/refresh`: a later gateway token for a session here.
///
/// @route POST /auth/central/refresh
/// @scope public
/// @operation-id `central_refresh`
/// @summary Carry a Central session on with a later token
/// @description Takes a later gateway token of a session this gateway holds:
/// the session lasts as long as the token, the token's terms stand, and a new
/// bearer comes back. It is also how a client whose renewal was lost gets back
/// in. A session this gateway does not hold, as after its restart, signs in.
/// @tag Central
/// @security none
/// @body `$CentralSignIn`
/// @return 200: `$CentralRefreshed` "The new bearer"
/// @return 400: "The body is no sign-in"
/// @return 401: "The token is not good here, its session was revoked, or this gateway holds no such session"
/// @return 413: String text/plain "The body is over 8 KiB"
/// @return 426: `$RefusalBody` "A client from before gateway tokens: `update_required`"
/// @return 429: "Too many sign-ins from this address"
/// @return 503: "No Baylee Central is configured, or this gateway has not caught up with Central since it started"
async fn refresh(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !within_budget(&state, addr, &headers) {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    match read_token(&body) {
        Ok(token) => answered(refreshed(&state, &token).await),
        Err(refused) => *refused,
    }
}

/// `POST /auth/session/renew`: a new bearer for the one presented.
///
/// @route POST /auth/session/renew
/// @scope public
/// @operation-id `session_renew`
/// @summary Renew a session's bearer
/// @description A Central session's bearer lives between 64 and 128 seconds.
/// This replaces the current one, with no word from Central; the one replaced
/// serves requests in flight for ten seconds more and renews nothing.
/// @tag Central
/// @security bearer
/// @return 200: `$CentralRenewed` "The new bearer"
/// @return 401: "No bearer, or not the current one of a live session"
async fn renew(State(state): State<Shared>, headers: HeaderMap) -> Response {
    let Some(bearer) = crate::bearer_token(&headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let digest = crate::auth::token_digest(bearer);
    match state.sessions.renew(&digest, sessions::now()) {
        Some(issued) => Json(json!({
            "token": issued.token,
            "expires_at": issued.expires_at,
            "renew_in": issued.renew_in,
        }))
        .into_response(),
        None => StatusCode::UNAUTHORIZED.into_response(),
    }
}

/// Ends the sessions a revocation names, which it has already written down.
fn end_sessions(book: &sessions::Book, sids: &[String]) {
    for sid in sids {
        book.end(&key_of(sid), Ended::Revoked);
    }
}

/// Applies a message that was taken: its sessions end now, and a gap sets the
/// catch-up going.
pub(super) fn ended(book: &sessions::Book, bridge: &Bridge, applied: &super::trust::Applied) {
    end_sessions(book, &applied.sids);
    if applied.gap {
        bridge.sync.notify_one();
    }
}

/// `POST /auth/central/invalidate`: Central ends sessions here.
///
/// @route POST /auth/central/invalidate
/// @scope public
/// @operation-id `central_invalidate`
/// @summary Take a revocation from Baylee Central
/// @description Anyone can reach this, so nothing is taken but a message
/// signed by Central's keys, for this gateway, unexpired and not taken before.
/// It names only `sid`s, and those sessions' sockets close at once. Nothing
/// here is needed for a revocation to arrive: the gateway also catches up on
/// its own.
/// @tag Central
/// @security none
/// @return 204: "Taken, or taken before"
/// @return 400: "Not `application/jwt`, or not text"
/// @return 401: "Refused"
/// @return 413: String text/plain "The body is over 16 KiB"
/// @return 429: "Too many from this address, or too many at once: try again"
/// @return 503: "No Baylee Central is configured, or this gateway holds no keys yet"
async fn invalidate(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    // Before a signature is checked, so a flood costs little.
    if !within_push_budget(&state, addr, &headers) {
        return StatusCode::TOO_MANY_REQUESTS;
    }
    let jwt = headers
        .get(CONTENT_TYPE)
        .is_some_and(|kind| kind.as_bytes() == b"application/jwt");
    let Some(token) = std::str::from_utf8(&body).ok().filter(|_| jwt) else {
        return StatusCode::BAD_REQUEST;
    };
    let Ok(bridge) = bridge(&state) else {
        return StatusCode::SERVICE_UNAVAILABLE;
    };
    let mut taken = bridge.trust.pushed(token, sessions::now());
    if matches!(taken, Err(Refused::UnknownKey)) {
        bridge.refresh_keys(true).await;
        taken = bridge.trust.pushed(token, sessions::now());
    }
    match taken {
        Ok(applied) => {
            ended(&state.sessions, &bridge, &applied);
            StatusCode::NO_CONTENT
        }
        Err(Refused::Replayed) => StatusCode::NO_CONTENT,
        Err(Refused::Full) => StatusCode::TOO_MANY_REQUESTS,
        Err(Refused::NotReady) => StatusCode::SERVICE_UNAVAILABLE,
        Err(_) => StatusCode::UNAUTHORIZED,
    }
}

/// The Central routes, outside the migration barrier (each takes it itself)
/// and with body limits of their own.
///
/// @openapi document
/// ```yaml
/// components:
///   schemas:
///     CentralSignIn:
///       type: object
///       description: 'A gateway token Baylee Central issued for this gateway: an EdDSA JWT, at most 2 KiB.'
///       properties:
///         token:
///           type: string
///       required: [token]
///       additionalProperties: false
///     WireRefusal:
///       type: object
///       description: 'Why, for a client to say in its own words; docs/protocol.md "Refusals".'
///       properties:
///         code:
///           type: string
///           pattern: '^[a-z0-9_]{1,32}$'
///         min_version:
///           type: string
///         until:
///           type: integer
///           format: int64
///         text:
///           type: string
///           maxLength: 200
///       required: [code]
///     RefusalBody:
///       type: object
///       description: 'A refusal and nothing else.'
///       properties:
///         refusal:
///           $ref: '#/components/schemas/WireRefusal'
///       required: [refusal]
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
///           description: 'When it ends, Unix seconds: the end of its gateway token.'
///         renew_in:
///           type: integer
///           description: 'Seconds from now to renew the bearer: relative, so that a client''s clock does not come into it.'
///         session_expires_at:
///           type: integer
///           format: int64
///           description: 'When the session ends, Unix seconds.'
///         username:
///           type: [string, 'null']
///           description: 'The account''s username here, `null` for an account only Central knows.'
///         display_name:
///           type: string
///           description: 'The name other players see.'
///       required: [token, expires_at, renew_in, session_expires_at, username, display_name]
///     CentralSession:
///       type: object
///       description: 'A session at this gateway, answered as a sign-in is.'
///       properties:
///         token:
///           type: string
///           description: 'The session''s bearer: 64 lowercase hex characters. It lives 64 to 128 seconds; renew it before `expires_at`.'
///         expires_at:
///           type: integer
///           format: int64
///           description: 'When the bearer ends, Unix seconds.'
///         renew_in:
///           type: integer
///           description: 'Seconds from now to renew the bearer: relative, so that a client''s clock does not come into it.'
///         session_expires_at:
///           type: integer
///           format: int64
///           description: 'When the session ends without a later token, Unix seconds.'
///         username:
///           type: [string, 'null']
///           description: 'The account''s username here, `null` for an account only Central knows.'
///         display_name:
///           type: string
///           description: 'The name other players see.'
///         terms_stale:
///           type: boolean
///           description: 'Whether the account has yet to accept this gateway''s terms in force (at Central); only on a gateway with terms.'
///       required: [token, expires_at, renew_in, session_expires_at, username, display_name]
///     CentralRefreshed:
///       type: object
///       description: 'A new bearer for a session carried on with a later token.'
///       properties:
///         token:
///           type: string
///         expires_at:
///           type: integer
///           format: int64
///         renew_in:
///           type: integer
///           description: 'Seconds from now to renew the bearer: relative, so that a client''s clock does not come into it.'
///         session_expires_at:
///           type: integer
///           format: int64
///         terms_stale:
///           type: boolean
///       required: [token, expires_at, renew_in, session_expires_at]
///     CentralRenewed:
///       type: object
///       description: 'A new bearer for the same session.'
///       properties:
///         token:
///           type: string
///         expires_at:
///           type: integer
///           format: int64
///         renew_in:
///           type: integer
///       required: [token, expires_at, renew_in]
/// ```
pub(crate) fn router() -> Router<Shared> {
    let sign_in = Router::new()
        .route("/auth/central/login", post(login))
        .route("/auth/central/admin", post(admin_login))
        .route("/auth/central/refresh", post(refresh))
        .route("/auth/session/renew", post(renew))
        .layer(DefaultBodyLimit::max(8192));
    let invalidating = Router::new()
        .route("/auth/central/invalidate", post(invalidate))
        .layer(DefaultBodyLimit::max(Kind::Revocation.max_bytes()));
    sign_in.merge(invalidating)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status_of(body: &[u8]) -> StatusCode {
        read_token(body).map_or_else(|refused| refused.status(), |_| StatusCode::OK)
    }

    #[test]
    fn a_sign_in_carries_a_token() {
        let token = read_token(br#"{"token":"a.b.c"}"#).ok().unwrap();
        assert_eq!(token.as_str(), "a.b.c");
    }

    /// `docs/protocol.md` §"Refusals": a client from before gateway tokens
    /// hears that it is too old, not that its request was bad.
    #[test]
    fn a_client_from_before_gateway_tokens_is_told_to_update() {
        let old = br#"{"ticket":[1,2,3]}"#;
        assert_eq!(status_of(old), StatusCode::UPGRADE_REQUIRED);
    }

    #[tokio::test]
    async fn being_told_to_update_says_so_as_a_refusal_a_client_can_read() {
        let refused = read_token(br#"{"ticket":[1]}"#).err().unwrap();
        let body = axum::body::to_bytes((*refused).into_body(), 1024)
            .await
            .unwrap();
        let refusal = Refusal::read(&body).unwrap();
        assert_eq!(refusal.code, "update_required");
    }

    #[test]
    fn anything_else_is_a_bad_request_and_never_taken_for_an_old_client() {
        for body in [
            &b"not json"[..],
            b"{}",
            br#"{"token":1}"#,
            br#"{"token":"a.b.c","more":1}"#,
        ] {
            assert_eq!(status_of(body), StatusCode::BAD_REQUEST);
        }
    }

    /// What the race below needs: a trust that holds keys, and what signs for
    /// them.
    mod race {
        use super::super::super::trust::Trust;
        use super::*;
        use central_crypto::jwt::{KeySet, RevocationClaims, SigningKey};
        use zeroize::Zeroizing;

        const ISSUER: &str = "https://central.example";
        const HERE: &str = "https://gateway.example";
        const NOW: i64 = 1_800_000_000;

        fn key() -> SigningKey {
            SigningKey::from_seed(&Zeroizing::new([7; 32]))
        }

        fn trust() -> Trust {
            let trust = Trust::new(ISSUER.into(), vec![HERE.into()]);
            let keys = KeySet::new(vec![key().public().clone()]).unwrap();
            trust.set_keys(keys, NOW);
            trust.caught_up();
            trust
        }

        fn sid(n: u64) -> String {
            format!("{n:032x}")
        }

        fn claims(n: u64) -> GatewayClaims {
            GatewayClaims {
                iss: ISSUER.into(),
                aud: HERE.into(),
                sub: "018f0000-0000-7000-8000-000000000001".into(),
                sid: sid(n),
                jti: format!("{:032x}", n + (1 << 40)),
                iat: NOW,
                nbf: NOW,
                exp: NOW + 3600,
                purpose: Purpose::Admit,
                name: None,
                tag: None,
                legacy_id: None,
                terms: None,
            }
        }

        /// Central's message revoking session `n`, as number `n`.
        fn revocation(n: u64) -> String {
            let claims = RevocationClaims {
                iss: ISSUER.into(),
                aud: HERE.into(),
                jti: format!("{n:032x}"),
                iat: NOW,
                nbf: NOW,
                exp: NOW + 60,
                from: n,
                seq: n,
                sids: vec![sid(n)],
            };
            key().sign(&claims).unwrap().to_string()
        }

        fn signed() -> Signed {
            Signed::new([1; 16], false, None)
        }

        fn open(book: &sessions::Book, trust: &Trust, n: u64) -> Result<(), StatusCode> {
            opened(book, trust, &claims(n), "a", NOW, signed()).map(drop)
        }

        fn revoke(book: &sessions::Book, trust: &Trust, n: u64) {
            let applied = trust.pushed(&revocation(n), NOW).ok().unwrap();
            end_sessions(book, &applied.sids);
        }

        /// The revocation lands in the waits of a sign-in: after its token was
        /// checked, before its session opened.
        #[test]
        fn a_revocation_before_the_session_opens_leaves_no_session() {
            let (book, trust) = (sessions::Book::default(), trust());
            trust
                .gateway_token(&key().sign(&claims(1)).unwrap(), NOW)
                .ok()
                .unwrap();
            revoke(&book, &trust, 1);
            assert_eq!(open(&book, &trust, 1), Err(StatusCode::UNAUTHORIZED));
            assert!(book.get(&key_of(&sid(1)), NOW).is_none());
        }

        #[test]
        fn a_revocation_after_the_session_opened_ends_it_and_its_holders_see() {
            let (book, trust) = (sessions::Book::default(), trust());
            let live = opened(&book, &trust, &claims(1), "a", NOW, signed()).unwrap();
            revoke(&book, &trust, 1);
            assert!(!live.stands(NOW));
            assert!(book.get(&key_of(&sid(1)), NOW).is_none());
        }

        /// The race itself, placed rather than hoped for: the whole revocation
        /// lands between the two steps of a sign-in. Two threads never meet
        /// here, since verifying a signature takes far longer than the window:
        /// a stress test passed with these two steps the wrong way round.
        /// Swap them and this fails.
        #[test]
        fn a_revocation_between_the_steps_of_a_sign_in_leaves_no_session() {
            let (book, trust) = (sessions::Book::default(), trust());
            let answer = opened_around(&book, &trust, &claims(1), "a", NOW, signed(), || {
                revoke(&book, &trust, 1);
            });
            assert_eq!(answer.err(), Some(StatusCode::UNAUTHORIZED));
            assert!(book.get(&key_of(&sid(1)), NOW).is_none());
        }

        /// The other side of the same order: a sign-in that lands between the
        /// two steps of a revocation. The `sid` is already written down, so it
        /// is refused, and nothing is left for the revocation to miss.
        #[test]
        fn a_sign_in_between_the_steps_of_a_revocation_is_refused() {
            let (book, trust) = (sessions::Book::default(), trust());
            let applied = trust.pushed(&revocation(1), NOW).ok().unwrap();
            assert_eq!(open(&book, &trust, 1), Err(StatusCode::UNAUTHORIZED));
            end_sessions(&book, &applied.sids);
            assert!(book.get(&key_of(&sid(1)), NOW).is_none());
        }
    }

    #[test]
    fn a_session_key_cannot_be_taken_for_a_local_digest() {
        let key = key_of("0123456789abcdef0123456789abcdef");
        assert_ne!(key.len(), 32);
        assert!(key.starts_with(b"central:"));
    }
}
