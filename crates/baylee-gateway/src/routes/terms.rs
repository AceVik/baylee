//! The terms of use (WG-1): the text, accepting it, and whether a sign-in
//! must ask (`terms.rs` reads the files, one or one per language).

use crate::{
    AppState, Deserialize, ErrorBody, HeaderMap, Json, Query, Shared, State, StatusCode, authed,
    db_down, err, store,
};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};

/// What a gateway without terms answers where terms would be.
const NO_TERMS: &str = "this gateway has no terms of use";

/// What `GET /terms` takes.
#[derive(Deserialize)]
pub(crate) struct TermsQuery {
    /// The client's interface language (`de`, `en`; a regional tag is read
    /// by its first part). Absent is English.
    lang: Option<String>,
}

/// `GET /terms?lang=de` → `{version, updated?, markdown, lang?}`.
///
/// Public, because the terms are what a player reads before agreeing to
/// anything, and a gateway list may show them before anything is saved.
/// `404` on a gateway without terms.
///
/// The text is in the asked language when the gateway has it, else in
/// English; `lang` (and `Content-Language`) says which it is. A gateway
/// with one file for everyone answers that file and names no language, so
/// its answer is what it was before there were languages. Only `?lang=`
/// decides, never `Accept-Language`: the interface's language is the
/// player's choice and may not be the browser's, and an answer that varies
/// by its address alone cannot be cached in the wrong language.
pub(crate) async fn terms(
    State(state): State<Shared>,
    Query(query): Query<TermsQuery>,
) -> Result<Response, (StatusCode, Json<ErrorBody>)> {
    let terms = state
        .terms
        .as_ref()
        .ok_or_else(|| err(StatusCode::NOT_FOUND, NO_TERMS))?;
    let text = terms.text(query.lang.as_deref());
    let mut body = serde_json::json!({
        "version": terms.version,
        "markdown": text.markdown,
    });
    if let Some(updated) = &text.updated {
        body["updated"] = updated.clone().into();
    }
    if let Some(lang) = &text.lang {
        body["lang"] = lang.clone().into();
    }
    let mut response = Json(body).into_response();
    if let Some(lang) = text
        .lang
        .as_deref()
        .and_then(|l| HeaderValue::from_str(l).ok())
    {
        response
            .headers_mut()
            .insert(header::CONTENT_LANGUAGE, lang);
    }
    Ok(response)
}

/// What `POST /account/terms` takes: the version the player read.
#[derive(Deserialize)]
pub(crate) struct AcceptTerms {
    /// As `GET /terms` named it.
    version: String,
}

/// `POST /account/terms {version}`: the signed-in account accepts the terms.
///
/// The version must be the current one: a client that showed an older text
/// (the file changed while the sheet was up) is answered `409` and asks
/// again, rather than recording acceptance of words the player never saw.
pub(crate) async fn accept_terms(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<AcceptTerms>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let terms = state
        .terms
        .as_ref()
        .ok_or_else(|| err(StatusCode::NOT_FOUND, NO_TERMS))?;
    if body.version != terms.version {
        return Err(err(
            StatusCode::CONFLICT,
            "the terms have changed since they were shown",
        ));
    }
    store::accept_terms(&state.db, &account_id, &terms.version)
        .await
        .map_err(|e| db_down(&e))?;
    Ok(Json(serde_json::json!({ "version": terms.version })))
}

/// Adds `terms_stale` to a sign-in's answer (login, register, guest): `true`
/// when the account last accepted another version than the current one, or
/// none. A gateway without terms adds nothing, so its answers are what they
/// were before terms existed.
pub(crate) fn mark_stale(state: &AppState, accepted: Option<&str>, answer: &mut serde_json::Value) {
    if let Some(terms) = &state.terms {
        answer["terms_stale"] = (accepted != Some(terms.version.as_str())).into();
    }
}
