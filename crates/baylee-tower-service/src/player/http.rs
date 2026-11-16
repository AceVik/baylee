use super::{Failure, Players};
use axum::{
    Router,
    body::to_bytes,
    extract::State,
    response::Response,
    routing::{get, post},
};
use baylee_tower_api as api;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) fn router(players: Arc<Players>) -> Router {
    Router::new()
        .route(api::PLAYER_PATH, post(player))
        .route(api::socket::PATH, get(super::socket::upgrade))
        .with_state(players)
}

async fn player(State(players): State<Arc<Players>>, request: axum::extract::Request) -> Response {
    let result = execute(&players, request).await;
    crate::http::reply(result)
}

async fn execute(players: &Players, request: axum::extract::Request) -> Result<Vec<u8>, Failure> {
    // Slots cover body reading, online introspection and the storage operation.
    let _slot = players.slots.try_acquire().map_err(|_| Failure::Busy)?;
    if !players
        .limits
        .lock()
        .map_err(|_| Failure::Unavailable)?
        .global(Instant::now())
    {
        return Err(Failure::Busy);
    }
    if request.uri().query().is_some() {
        return Err(Failure::Invalid);
    }
    let (parts, body) = request.into_parts();
    let token = crate::http::credential(&parts.headers, "application/json")?;
    let bytes = tokio::time::timeout(
        Duration::from_secs(3),
        to_bytes(body, api::MAX_REQUEST_BYTES),
    )
    .await
    .map_err(|_| Failure::Invalid)?
    .map_err(|_| Failure::Oversized)?;
    let request = api::decode(&bytes).map_err(|_| Failure::Invalid)?;
    let identity = players.central.authenticate(&token).await?;
    let account = uuid::Uuid::from_bytes(identity.account);
    if !players
        .limits
        .lock()
        .map_err(|_| Failure::Unavailable)?
        .account(account, Instant::now())
    {
        return Err(Failure::Busy);
    }
    // A client may retry a mutation with the same request ID after any lost answer.
    // Never timeout-cancel its commit and invent a new request on the client's behalf.
    let name = identity
        .display_name
        .zip(identity.tag)
        .map(|(display, tag)| api::ranking::Name { display, tag });
    let response = players.dispatch(account, request, name.as_ref()).await?;
    let encoded = serde_json::to_vec(&response).map_err(|_| Failure::Unavailable)?;
    if encoded.len() > api::MAX_RESPONSE_BYTES {
        return Err(Failure::Unavailable);
    }
    Ok(encoded)
}
