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

fn bridge(state: &Shared) -> Result<&Arc<Bridge>, StatusCode> {
    state
        .central
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
}

fn migration_bridge(state: &Shared) -> Result<&Arc<Bridge>, StatusCode> {
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
    let result = bridge
        .exchange(Request::LegacyReserve {
            legacy_id: account.id,
            username: source.username.clone(),
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

async fn login(
    State(state): State<Shared>,
    Json(capability): Json<Capability>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    login_for(&state, capability, TicketPurpose::Admit).await
}

async fn admin_login(
    State(state): State<Shared>,
    Json(capability): Json<Capability>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if bridge(&state)?.legacy {
        return Err(StatusCode::FORBIDDEN);
    }
    login_for(&state, capability, TicketPurpose::Admin).await
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
    let account_id = if bridge(state)?.legacy {
        let legacy = legacy.ok_or(StatusCode::UNAUTHORIZED)?;
        if !baylee_db::central::matches(&state.db, &legacy, identity.account)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        {
            return Err(StatusCode::UNAUTHORIZED);
        }
        legacy
    } else {
        baylee_db::central::projection(
            &state.db,
            identity.account,
            matches!(purpose, TicketPurpose::Admin)
                || state.registration == crate::invite::Registration::Open,
        )
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    };
    let account = allowed(state, &account_id).await?;
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
    crate::routes::mark_stale(state, account.terms_version.as_deref(), &mut answer);
    Ok(Json(answer))
}

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
