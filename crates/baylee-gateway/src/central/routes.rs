use super::{Bridge, Shared};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    response::IntoResponse as _,
    routing::post,
};
use baylee_account::{
    migration::{
        Capability, Credentials, Done, Enrollment, FreshProofRequired, Link, Preparation, Snapshot,
    },
    wire::{Identity, Request, Response, TicketPurpose},
};
use std::{sync::Arc, time::Duration};

fn bridge(state: &Shared) -> Result<Arc<Bridge>, StatusCode> {
    state.central.get().ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

fn migration_bridge(state: &Shared) -> Result<Arc<Bridge>, StatusCode> {
    let bridge = bridge(state)?;
    if !bridge.legacy {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(bridge)
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

async fn snapshot(
    state: &Shared,
    account: &crate::store::Account,
    fresh_proof: bool,
) -> Result<Snapshot, StatusCode> {
    let snapshot = if fresh_proof {
        baylee_db::central::prepare(&state.db, &account.id).await
    } else {
        baylee_db::central::refresh(&state.db, &account.id).await
    }
    .map_err(|_| StatusCode::CONFLICT)?;
    let mut source = snapshot.document;
    let preferences = source
        .get_mut("preferences")
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .take();
    let decks = source
        .get_mut("decks")
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .take();
    Ok(Snapshot {
        digest: snapshot.digest,
        account_id: account.id.clone(),
        username: account.username.clone().ok_or(StatusCode::UNAUTHORIZED)?,
        preferences,
        decks,
    })
}

/// A legacy account's display name and tag as Central keeps them: the name
/// only when it keeps the rule, which one written before the rule of
/// 15.09.2026 (or imported from `gateway-store.json`) may not, and which
/// Central refuses; such an account keeps its tag and chooses a name on the
/// website. Central's check is the same rule (`central_api::profile`), held
/// to the same table of cases in `auth.rs`.
fn profile(account: &crate::store::Account) -> (Option<String>, Option<u32>) {
    let name = crate::auth::valid_display_name(&account.display_name)
        .then(|| account.display_name.clone());
    (name, u32::try_from(account.tag).ok().filter(|tag| *tag > 0))
}

/// Carries a moved account's name and tag to Central once, for an account
/// that moved before Central kept them: at each of its Central sign-ins
/// here, since nothing on this side records that it was done. Central sets
/// them only while the account has no tag (`LegacyProfile`, `Done`) and
/// answers `Conflict` once it has, so a repeat changes nothing. Any answer,
/// a refusal or Central being away included, leaves the sign-in as it is:
/// the next one tries again.
async fn carry_profile(state: &Shared, account: &crate::store::Account) {
    let (display_name, Some(tag)) = profile(account) else {
        return;
    };
    let Ok(bridge) = bridge(state) else {
        return;
    };
    let _ = bridge
        .exchange(Request::LegacyProfile {
            legacy_id: account.id.clone(),
            display_name,
            tag,
        })
        .await;
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

/// `POST /auth/central/migrate`: a legacy account's last password proof,
/// answered with its reserved Central identity and its source snapshot.
///
/// @route POST /auth/central/migrate
/// @scope public
/// @operation-id `central_migrate`
/// @summary Begin moving a legacy account to Baylee Central
/// @description The native client's last password proof for a legacy
/// account, on the official migration gateway only. The gateway checks the
/// password, a ban and a forced reset, reserves the account's immutable
/// Central identity, and answers it with an enrollment ticket and the
/// account's source snapshot. Every refusal is a bare status.
/// @tag Central
/// @security none
/// @body `$CentralCredentials`
/// @return 200: `$CentralEnrollment` "The reserved identity, its enrollment ticket and the source snapshot"
/// @return 401: "A wrong username or password, an account linked to Central already, a guest, a password or name that changed meanwhile, or Central refused"
/// @return 403: "This gateway is not the migration gateway, or the account is banned or must choose a new password first"
/// @return 409: "The snapshot could not be prepared (it is too large), or Central holds a conflicting enrollment"
/// @return 413: String text/plain "The body is over 8 KiB"
/// @return 429: "Too many sign-in attempts at this account, or Central is busy"
/// @return 500: "The password check failed"
/// @return 503: "No Baylee Central is configured, or the database or Central is unavailable"
async fn migrate(
    State(state): State<Shared>,
    Json(credentials): Json<Credentials>,
) -> Result<Json<Enrollment>, StatusCode> {
    let bridge = migration_bridge(&state)?;
    let proven = crate::routes::proven(&state, &credentials.username, &credentials.password)
        .await
        .map_err(|e| e.0)?;
    drop(credentials);
    let _guard = exclusive(&state).await?;
    let account = allowed(&state, &proven.id).await?;
    // A password/name change may have finished while verification ran outside
    // the exclusive barrier. Its old proof must not enroll the changed account.
    if account.password_hash != proven.password_hash || account.username != proven.username {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let source = snapshot(&state, &account, true).await?;
    // The moved account keeps its name and tag here at Central
    // (baylee-central `docs/native-login.md`).
    let (display_name, tag) = profile(&account);
    let result = bridge
        .exchange(Request::LegacyReserve {
            legacy_id: account.id,
            username: source.username.clone(),
            display_name,
            tag,
        })
        .await?;
    match result {
        Response::Enrollment { identity, ticket } => Ok(Json(Enrollment {
            identity,
            ticket,
            snapshot: source,
        })),
        _ => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

/// `POST /auth/central/export`: the source snapshot again, for a migration
/// that was interrupted.
///
/// @route POST /auth/central/export
/// @scope public
/// @operation-id `central_export`
/// @summary Read a migrating account's source snapshot again
/// @description Resumes an interrupted migration on the official migration
/// gateway: an `export` ticket from Baylee Central buys the legacy
/// account's source snapshot as it stands now. Every refusal is a bare
/// status.
/// @tag Central
/// @security none
/// @body `$CentralCapability`
/// @return 200: `$CentralSnapshot` "The account's source snapshot"
/// @return 401: "The ticket is not good, names no legacy account, or the account is a guest or has no username"
/// @return 403: "This gateway is not the migration gateway, or the account is banned or must choose a new password first"
/// @return 409: "The snapshot could not be read, or Central answered a conflict"
/// @return 413: String text/plain "The body is over 8 KiB"
/// @return 429: "Central is busy"
/// @return 503: "No Baylee Central is configured, or the database or Central is unavailable"
async fn export(
    State(state): State<Shared>,
    Json(capability): Json<Capability>,
) -> Result<Json<Snapshot>, StatusCode> {
    migration_bridge(&state)?;
    let (_, legacy, _, _) = redeem(&state, capability, TicketPurpose::Export).await?;
    let legacy = legacy.ok_or(StatusCode::UNAUTHORIZED)?;
    let _guard = exclusive(&state).await?;
    let account = allowed(&state, &legacy).await?;
    Ok(Json(snapshot(&state, &account, false).await?))
}

/// `POST /auth/central/link`: the cutover, once the client restored the
/// snapshot at Central.
///
/// @route POST /auth/central/link
/// @scope public
/// @operation-id `central_link`
/// @summary Link a migrated account to Baylee Central
/// @description Finishes a migration on the official migration gateway: a
/// `link` ticket and the digest of the snapshot the client restored fix the
/// legacy account's link to its Central identity, retire its password here
/// and end its old sessions. Answers `428` when the gateway holds no source
/// proof for the account yet: then, and only then, `POST
/// /auth/central/prepare` takes one fresh password proof. Every other
/// refusal is a bare status.
/// @tag Central
/// @security none
/// @body `$CentralLink`
/// @return 200: `$CentralDone` "Linked; the legacy account's sessions are ended"
/// @return 401: "The ticket is not good or names no legacy account, or the account is a guest"
/// @return 403: "This gateway is not the migration gateway, or the account is banned or must choose a new password first"
/// @return 409: "The digest is not the snapshot's, the source changed, or Central answered a conflict"
/// @return 413: String text/plain "The body is over 8 KiB"
/// @return 428: `$CentralFreshProofRequired` "No source proof is held for this account: send one with `POST /auth/central/prepare`"
/// @return 429: "Central is busy"
/// @return 503: "No Baylee Central is configured, or the database or Central is unavailable"
async fn link(
    State(state): State<Shared>,
    Json(link): Json<Link>,
) -> Result<axum::response::Response, StatusCode> {
    migration_bridge(&state)?;
    let (identity, legacy, _, _) = redeem(
        &state,
        Capability {
            ticket: link.ticket,
        },
        TicketPurpose::Link,
    )
    .await?;
    let legacy = legacy.ok_or(StatusCode::UNAUTHORIZED)?;
    let _guard = exclusive(&state).await?;
    allowed(&state, &legacy).await?;
    if !baylee_db::central::linked(&state.db, &legacy)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        && !baylee_db::central::prepared(&state.db, &legacy)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    {
        return Ok((
            StatusCode::PRECONDITION_REQUIRED,
            Json(FreshProofRequired {
                fresh_proof_required: true,
            }),
        )
            .into_response());
    }
    baylee_db::central::validate_cutover(&state.db, &legacy, identity.account, link.digest)
        .await
        .map_err(|_| StatusCode::CONFLICT)?;
    if !matches!(
        migration_bridge(&state)?
            .exchange(Request::LegacyClose {
                legacy_id: legacy.clone(),
                identity,
            })
            .await?,
        Response::Done
    ) {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    baylee_db::central::finish(&state.db, &legacy, identity.account, link.digest)
        .await
        .map_err(|_| StatusCode::CONFLICT)?;
    // Existing authenticated lobby/chat sockets must reauthenticate too.
    let _ = state.departed.send(legacy);
    Ok(Json(Done {}).into_response())
}

/// `POST /auth/central/prepare`: one fresh password proof for a migration
/// from before the source proof was kept, after a `428` from
/// `POST /auth/central/link`.
///
/// @route POST /auth/central/prepare
/// @scope public
/// @operation-id `central_prepare`
/// @summary Prove a migrating account's password again
/// @description After `POST /auth/central/link` answered `428`: a `link`
/// ticket and the legacy account's password establish the source proof the
/// gateway lacks, and the answer is the account's source snapshot. Only an
/// account with no source proof yet is taken. Every refusal is a bare
/// status.
/// @tag Central
/// @security none
/// @body `$CentralPreparation`
/// @return 200: `$CentralSnapshot` "The account's source snapshot"
/// @return 401: "The ticket is not good or is another account's, a wrong username or password, an account linked already, a guest, or a password or name that changed meanwhile"
/// @return 403: "This gateway is not the migration gateway, or the account is banned or must choose a new password first"
/// @return 409: "A source proof is held already, the snapshot could not be prepared, or Central answered a conflict"
/// @return 413: String text/plain "The body is over 8 KiB"
/// @return 429: "Too many sign-in attempts at this account, or Central is busy"
/// @return 500: "The password check failed"
/// @return 503: "No Baylee Central is configured, or the database or Central is unavailable"
async fn prepare(
    State(state): State<Shared>,
    Json(preparation): Json<Preparation>,
) -> Result<Json<Snapshot>, StatusCode> {
    migration_bridge(&state)?;
    let (_, legacy, _, _) = redeem(
        &state,
        Capability {
            ticket: preparation.ticket,
        },
        TicketPurpose::Link,
    )
    .await?;
    let legacy = legacy.ok_or(StatusCode::UNAUTHORIZED)?;
    let proven = crate::routes::proven(
        &state,
        &preparation.credentials.username,
        &preparation.credentials.password,
    )
    .await
    .map_err(|error| error.0)?;
    drop(preparation.credentials);
    if proven.id != legacy {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let _guard = exclusive(&state).await?;
    let account = allowed(&state, &legacy).await?;
    if account.password_hash != proven.password_hash || account.username != proven.username {
        return Err(StatusCode::UNAUTHORIZED);
    }
    // Only repairs an absent preparation; an existing source cannot be reauthorized
    // through this recovery operation after a password mutation or conflicting export.
    if baylee_db::central::prepared(&state.db, &legacy)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    {
        return Err(StatusCode::CONFLICT);
    }
    Ok(Json(snapshot(&state, &account, true).await?))
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
/// @return 403: "The account is banned or must choose a new password first, or Central refused"
/// @return 409: "Central answered a conflict"
/// @return 413: String text/plain "The body is over 8 KiB"
/// @return 429: "Central is busy"
/// @return 503: "No Baylee Central is configured, the database or Central is unavailable, or the account is new here and this gateway's registration is not open"
async fn login(
    State(state): State<Shared>,
    Json(capability): Json<Capability>,
) -> Result<Json<serde_json::Value>, StatusCode> {
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
    let legacy_link = legacy.is_some();
    let account_id = match (bridge(state)?.legacy, legacy) {
        // The official gateway: a migrated account is the legacy account it
        // was linked to, and nothing else.
        (true, Some(legacy)) => {
            if !baylee_db::central::matches(&state.db, &legacy, identity.account)
                .await
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            {
                return Err(StatusCode::UNAUTHORIZED);
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
    if bridge(state)?.legacy && legacy_link {
        carry_profile(state, &account).await;
    } else if take_profile(state, &lease, purpose, identity.account, &account_id).await {
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
/// The bodies and answers are `baylee_account::migration`'s and
/// `central_api`'s types, which the API document is not read from, so their
/// schemas are spelled here. Each `Token` and digest is 32 bytes as a JSON
/// array of numbers (`[u8; 32]`), an identity's account 16.
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
///     CentralIdentity:
///       type: object
///       description: 'An account''s immutable identity at Baylee Central.'
///       properties:
///         account:
///           type: array
///           description: 'The Central account''s UUIDv7: its 16 bytes, as a JSON array of numbers from 0 to 255.'
///           items: { type: integer, minimum: 0, maximum: 255 }
///           minItems: 16
///           maxItems: 16
///         epoch:
///           type: integer
///           format: int64
///           minimum: 0
///           description: 'The epoch of the account''s encrypted vault at Central.'
///       required: [account, epoch]
///       additionalProperties: false
///     CentralCredentials:
///       type: object
///       description: 'A legacy account''s last password proof; never kept.'
///       properties:
///         username:
///           type: string
///           description: 'The legacy account''s username.'
///         password:
///           type: string
///           description: 'Its password.'
///       required: [username, password]
///       additionalProperties: false
///     CentralCapability:
///       type: object
///       description: 'A one-use ticket Baylee Central issued for this gateway and this route''s purpose, good for 30 seconds.'
///       properties:
///         ticket:
///           $ref: '#/components/schemas/CentralBytes32'
///       required: [ticket]
///       additionalProperties: false
///     CentralLink:
///       type: object
///       description: 'What links a migrated account.'
///       properties:
///         ticket:
///           $ref: '#/components/schemas/CentralBytes32'
///           description: 'A one-use `link` ticket from Baylee Central.'
///         digest:
///           $ref: '#/components/schemas/CentralBytes32'
///           description: 'The digest of the source snapshot the client restored, as the snapshot named it.'
///       required: [ticket, digest]
///       additionalProperties: false
///     CentralPreparation:
///       type: object
///       description: 'A fresh source proof for a migration, bound to a `link` ticket.'
///       properties:
///         ticket:
///           $ref: '#/components/schemas/CentralBytes32'
///           description: 'A one-use `link` ticket from Baylee Central.'
///         credentials:
///           $ref: '#/components/schemas/CentralCredentials'
///       required: [ticket, credentials]
///       additionalProperties: false
///     CentralSnapshot:
///       type: object
///       description: 'A legacy account''s personal content as this gateway holds it; the gateway keeps its own copy.'
///       properties:
///         digest:
///           $ref: '#/components/schemas/CentralBytes32'
///           description: 'The digest the cutover is bound to; `POST /auth/central/link` names it back.'
///         account_id:
///           type: string
///           description: 'The legacy account''s immutable id here.'
///         username:
///           type: string
///           description: 'Its username.'
///         preferences:
///           type: object
///           description: 'Its whole preferences document, fields this client does not know included.'
///         decks:
///           type: array
///           description: 'Its decks, each with its whole version history (`history`).'
///           items: { type: object }
///       required: [digest, account_id, username, preferences, decks]
///       additionalProperties: false
///     CentralEnrollment:
///       type: object
///       description: 'The start of a migration.'
///       properties:
///         identity:
///           $ref: '#/components/schemas/CentralIdentity'
///         ticket:
///           $ref: '#/components/schemas/CentralBytes32'
///           description: 'The enrollment ticket for Baylee Central, which cannot replace a credential that exists already.'
///         snapshot:
///           $ref: '#/components/schemas/CentralSnapshot'
///       required: [identity, ticket, snapshot]
///       additionalProperties: false
///     CentralDone:
///       type: object
///       description: 'Done; nothing else is said.'
///       additionalProperties: false
///     CentralFreshProofRequired:
///       type: object
///       description: 'The one answer that lets a client send a fresh password proof (`POST /auth/central/prepare`).'
///       properties:
///         fresh_proof_required:
///           type: boolean
///           const: true
///           description: 'Always `true`.'
///       required: [fresh_proof_required]
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
        .route("/auth/central/migrate", post(migrate))
        .route("/auth/central/export", post(export))
        .route("/auth/central/link", post(link))
        .route("/auth/central/prepare", post(prepare))
        .route("/auth/central/login", post(login))
        .route("/auth/central/admin", post(admin_login))
        .layer(DefaultBodyLimit::max(8192))
}
