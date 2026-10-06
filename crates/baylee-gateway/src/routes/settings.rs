//! A player's stored preferences.

use crate::{ErrorBody, HeaderMap, Json, Shared, State, StatusCode, authed, db_down, err, store};

/// Upper bound on a stored preferences blob.
///
/// A full keymap with every action bound twice, both phase rails and the
/// automation flags is under two kilobytes; sixteen leaves room for whatever
/// the client learns to remember next, and still means a thousand accounts
/// cost the store sixteen megabytes at the very worst.
pub(crate) const MAX_SETTINGS_BYTES: usize = 16 * 1024;

/// The account's client preferences, verbatim as they were stored.
///
/// `{}` for an account that has never saved any, which is the same thing the
/// client would do with them: fall back to its own defaults.
pub(crate) async fn get_settings(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let settings = store::settings_of(&state.db, &account_id)
        .await
        .map_err(|e| db_down(&e))?
        .unwrap_or_else(|| serde_json::json!({}));
    Ok(Json(settings))
}

/// Replaces the account's client preferences.
///
/// The body *is* the preferences object — there is no wrapper, because there
/// is nothing else to say about it. The gateway checks only the two things it
/// can check without knowing what a keymap is: that this is an object, and
/// that it is not being used as free storage.
pub(crate) async fn put_settings(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    if !body.is_object() {
        return Err(err(StatusCode::BAD_REQUEST, "settings must be an object"));
    }
    let bytes = serde_json::to_string(&body).map_or(usize::MAX, |s| s.len());
    if bytes > MAX_SETTINGS_BYTES {
        return Err(err(StatusCode::PAYLOAD_TOO_LARGE, "settings too large"));
    }
    store::put_settings(&state.db, &account_id, body)
        .await
        .map_err(|e| db_down(&e))?;
    Ok(Json(serde_json::json!({ "stored": bytes })))
}
