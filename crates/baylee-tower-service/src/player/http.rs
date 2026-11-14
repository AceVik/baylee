use super::{Failure, Players};
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use baylee_tower_api as api;
use central_api::Token;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub(super) fn router(players: Arc<Players>) -> Router {
    Router::new()
        .route(api::PLAYER_PATH, post(player))
        .with_state(players)
}

async fn player(State(players): State<Arc<Players>>, request: axum::extract::Request) -> Response {
    let result = execute(&players, request).await;
    let mut response = match result {
        Ok(bytes) => ([(header::CONTENT_TYPE, "application/json")], bytes).into_response(),
        Err(failure) => failure.into_response(),
    };
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );
    response.headers_mut().insert(
        "x-content-type-options",
        header::HeaderValue::from_static("nosniff"),
    );
    response.headers_mut().insert(
        api::VERSION_HEADER,
        header::HeaderValue::from_static(api::VERSION),
    );
    response
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
    let token = credential(&parts.headers)?;
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
    let response = players.dispatch(account, request).await?;
    let encoded = serde_json::to_vec(&response).map_err(|_| Failure::Unavailable)?;
    if encoded.len() > api::MAX_RESPONSE_BYTES {
        return Err(Failure::Unavailable);
    }
    Ok(encoded)
}

fn credential(headers: &HeaderMap) -> Result<Token, Failure> {
    if headers.contains_key(header::ORIGIN)
        || headers.contains_key(header::COOKIE)
        || headers.contains_key(header::CONTENT_ENCODING)
    {
        return Err(Failure::Invalid);
    }
    if one(headers, api::VERSION_HEADER) != Some(api::VERSION) {
        return Err(Failure::Version);
    }
    if one(headers, "content-type") != Some("application/json") {
        return Err(Failure::Invalid);
    }
    Token::from_bearer(one(headers, "authorization").ok_or(Failure::Unauthorized)?)
        .map_err(|_| Failure::Unauthorized)
}

fn one<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    values.next().is_none().then_some(value)
}

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        let status = match self {
            Self::Invalid => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Terms => StatusCode::FORBIDDEN,
            Self::Missing => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Busy => StatusCode::TOO_MANY_REQUESTS,
            Self::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::Version => StatusCode::UPGRADE_REQUIRED,
            Self::Oversized => StatusCode::PAYLOAD_TOO_LARGE,
        };
        let mut response = Response::new(Body::empty());
        *response.status_mut() = status;
        if self == Self::Busy {
            response
                .headers_mut()
                .insert(header::RETRY_AFTER, header::HeaderValue::from_static("1"));
        }
        response
    }
}
