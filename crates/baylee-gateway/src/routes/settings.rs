//! A player's stored preferences.

use crate::{
    ErrorBody, HeaderMap, Json, Serialize, Shared, State, StatusCode, authed, db_down, err, store,
};

/// Upper bound on a stored preferences blob.
///
/// A full keymap with every action bound twice, both phase rails and the
/// automation flags is under two kilobytes; sixteen leaves room for whatever
/// the client learns to remember next, and still means a thousand accounts
/// cost the store sixteen megabytes at the very worst. It bounds the merged
/// document, not only one save's patch.
pub(crate) const MAX_SETTINGS_BYTES: usize = 16 * 1024;

/// The account's client preferences, as every save so far has merged them.
///
/// `{}` for an account that has never saved any, which is the same thing the
/// client would do with them: fall back to its own defaults.
///
/// @route GET /settings
/// @scope public
/// @summary The account's client preferences
/// @description The preferences the account's clients stored, as every save
/// so far has merged them: an object the gateway keeps without reading it.
/// `{}` for an account that has never saved any, never a `404`.
/// @tag Accounts
/// @security bearer
/// @insert `SessionRefusals`
/// @return 200: `{ type: object, additionalProperties: true }` "The stored preferences, `{}` when there are none"
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

/// Merges a patch into the account's client preferences (M4-7, WG-5).
///
/// The body is an object of top-level keys, with no wrapper. Each key it
/// names replaces that key's value whole; a key set to `null` is removed; a
/// key it does not name is kept ([`store::merge_settings`]). So a client
/// that does not know a preference cannot erase it by leaving it out, which
/// is what lets two client versions share one account. The gateway still
/// checks only what it can check without knowing what a keymap is: that the
/// body is an object, and that neither it nor the merged document is being
/// used as free storage. The answer names the merged document's size.
///
/// @route PUT /settings
/// @scope public
/// @summary Merge into the account's client preferences
/// @description Merges an object of top-level keys into the stored
/// preferences: each key named replaces that key's value whole, a key set
/// to `null` is removed, and a key not named is kept. The patch and the
/// merged object must each stay within 16 KiB of JSON. The answer names the
/// merged object's size.
/// @tag Accounts
/// @security bearer
/// @insert `SessionRefusals`
/// @body `{ type: object, additionalProperties: true }` "Top-level keys to replace; `null` removes a key"
/// @return 400: `$ErrorBody` "The body is not a JSON object"
/// @return 413: `$ErrorBody` "The patch or the merged preferences are over 16 KiB"
pub(crate) async fn put_settings(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<SettingsStored>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let serde_json::Value::Object(patch) = body else {
        return Err(err(StatusCode::BAD_REQUEST, "settings must be an object"));
    };
    let too_large = || err(StatusCode::PAYLOAD_TOO_LARGE, "settings too large");
    // The patch alone first, so an oversized body costs no transaction.
    let bytes = serde_json::to_string(&patch).map_or(usize::MAX, |s| s.len());
    if bytes > MAX_SETTINGS_BYTES {
        return Err(too_large());
    }
    match store::merge_settings(&state.db, &account_id, patch, MAX_SETTINGS_BYTES)
        .await
        .map_err(|e| db_down(&e))?
    {
        store::SettingsMerge::Stored(stored) => Ok(Json(SettingsStored { stored })),
        store::SettingsMerge::TooLarge => Err(too_large()),
    }
}

/// What `PUT /settings` answers.
/// @openapi
#[derive(Serialize)]
pub(crate) struct SettingsStored {
    /// @description The merged preferences' size, in bytes of JSON.
    stored: usize,
}
