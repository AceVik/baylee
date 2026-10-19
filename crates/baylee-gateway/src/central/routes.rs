use super::{Bridge, Shared};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::post,
};
use baylee_account::{
    migration::{Capability, Credentials, Done, Enrollment, Link, Snapshot},
    wire::{Identity, Request, Response, TicketPurpose},
};
use std::{sync::Arc, time::Duration};

fn bridge(state: &Shared) -> Result<&Arc<Bridge>, StatusCode> {
    state
        .central
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)
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

async fn snapshot(state: &Shared, account: &crate::store::Account) -> Result<Snapshot, StatusCode> {
    let snapshot = baylee_db::central::prepare(&state.db, &account.id)
        .await
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
) -> Result<(Identity, String), StatusCode> {
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
        } => Ok((identity, legacy_id)),
        _ => Err(StatusCode::SERVICE_UNAVAILABLE),
    }
}

async fn migrate(
    State(state): State<Shared>,
    Json(credentials): Json<Credentials>,
) -> Result<Json<Enrollment>, StatusCode> {
    let bridge = bridge(&state)?;
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
    let source = snapshot(&state, &account).await?;
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
    let (_, legacy) = redeem(&state, capability, TicketPurpose::Export).await?;
    let _guard = exclusive(&state).await?;
    let account = allowed(&state, &legacy).await?;
    Ok(Json(snapshot(&state, &account).await?))
}

async fn link(
    State(state): State<Shared>,
    Json(link): Json<Link>,
) -> Result<Json<Done>, StatusCode> {
    let (identity, legacy) = redeem(
        &state,
        Capability {
            ticket: link.ticket,
        },
        TicketPurpose::Link,
    )
    .await?;
    let _guard = exclusive(&state).await?;
    allowed(&state, &legacy).await?;
    baylee_db::central::finish(&state.db, &legacy, identity.account, link.digest)
        .await
        .map_err(|_| StatusCode::CONFLICT)?;
    // Existing authenticated lobby/chat sockets must reauthenticate too.
    let _ = state.departed.send(legacy);
    Ok(Json(Done {}))
}

async fn login(
    State(state): State<Shared>,
    Json(capability): Json<Capability>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (identity, legacy) = redeem(&state, capability, TicketPurpose::Admit).await?;
    let _guard = exclusive(&state).await?;
    let account = allowed(&state, &legacy).await?;
    if !baylee_db::central::matches(&state.db, &legacy, identity.account)
        .await
        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    crate::routes::signed_in(&state, &account)
        .await
        .map_err(|e| e.0)
}

pub(crate) fn router() -> Router<Shared> {
    Router::new()
        .route("/auth/central/migrate", post(migrate))
        .route("/auth/central/export", post(export))
        .route("/auth/central/link", post(link))
        .route("/auth/central/login", post(login))
        .layer(DefaultBodyLimit::max(8192))
}
