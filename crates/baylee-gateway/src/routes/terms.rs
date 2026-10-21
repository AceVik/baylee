//! The terms of use (WG-1): the text, accepting it, and whether a sign-in
//! must ask (`terms.rs` reads the files, one or one per language).

use crate::{
    AppState, Deserialize, ErrorBody, HeaderMap, Json, Query, Serialize, Shared, State, StatusCode,
    db_down, err, store,
};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};

/// What a gateway without terms answers where terms would be.
const NO_TERMS: &str = "this gateway has no terms of use";

/// What `GET /terms` takes.
/// @openapi
#[derive(Deserialize)]
pub(crate) struct TermsQuery {
    /// @description The client's interface language (`de`, `en`; a regional
    /// tag such as `de-AT` is read by its first part). Absent is English.
    /// @internal
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
///
/// @route GET /terms
/// @scope public
/// @summary The terms of use in force
/// @description The terms a player reads before accepting them: the
/// version, its digest and the text, in the asked language when the gateway
/// has it, else in English. `lang` and `Content-Language` say which language
/// it is; a gateway with one text for every language names none. Only
/// `?lang=` decides, never `Accept-Language`.
/// @tag Terms
/// @security none
/// @query-params `$TermsQuery`
/// @return 200: `$TermsDocument` "The terms in force"
/// @return-header 200 Content-Language: Option<String> "The language of the text, when the gateway names one"
/// @return 404: `$ErrorBody` "This gateway has no terms of use"
pub(crate) async fn terms(
    State(state): State<Shared>,
    Query(query): Query<TermsQuery>,
) -> Result<Response, (StatusCode, Json<ErrorBody>)> {
    let terms = state
        .terms
        .current()
        .ok_or_else(|| err(StatusCode::NOT_FOUND, NO_TERMS))?;
    let text = terms.text(query.lang.as_deref());
    let body = TermsDocument {
        version: terms.version.clone(),
        digest: crate::auth::hex_lower(&terms.digest),
        markdown: text.markdown.clone(),
        updated: text.updated.clone(),
        lang: text.lang.clone(),
    };
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

/// What `GET /terms` answers.
/// @openapi
#[derive(Serialize)]
pub(crate) struct TermsDocument {
    /// @description The version an account accepts, the same in every
    /// language.
    version: String,
    /// @description The digest of every language's text, 64 lowercase hex
    /// digits: what an acceptance at Baylee Central names beside the
    /// version.
    digest: String,
    /// @description The text, as Markdown.
    markdown: String,
    /// @description The date the text says it was last changed
    /// (`2026-10-06`), when it says.
    #[serde(skip_serializing_if = "Option::is_none")]
    updated: Option<String>,
    /// @description The language the text is in, a primary language tag
    /// (`de`); absent when the gateway has one text for every language.
    #[serde(skip_serializing_if = "Option::is_none")]
    lang: Option<String>,
}

/// What `POST /account/terms` takes: the version the player read.
/// @openapi
#[derive(Deserialize)]
pub(crate) struct AcceptTerms {
    /// @description The version the player read, as `GET /terms` named it.
    /// @internal
    /// As `GET /terms` named it.
    version: String,
}

/// What `POST /account/terms` answers.
/// @openapi
#[derive(Serialize)]
pub(crate) struct TermsAccepted {
    /// @description The version the account has now accepted.
    version: String,
}

/// `POST /account/terms {version}`: the signed-in account accepts the terms.
///
/// The version must be the current one: a client that showed an older text
/// (the file changed while the sheet was up) is answered `409` and asks
/// again, rather than recording acceptance of words the player never saw.
///
/// @route POST /account/terms
/// @scope public
/// @summary Accept the terms of use
/// @description Records that the signed-in account accepted the terms in
/// force, by the version `GET /terms` named. A version that is no longer
/// current is `409`, and the client shows the new text. An account of
/// Baylee Central accepts at Central instead, and is answered `409` here.
/// Open to a session that has not accepted the terms in force.
/// @tag Terms
/// @security bearer
/// @insert `SessionRefusalsAnyTerms`
/// @return 404: `$ErrorBody` "This gateway has no terms of use"
/// @return 409: `$ErrorBody` "The terms changed since they were shown, or the account accepts them at Baylee Central"
pub(crate) async fn accept_terms(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<AcceptTerms>,
) -> Result<Json<TermsAccepted>, (StatusCode, Json<ErrorBody>)> {
    let session = crate::authed_session_any_terms(&state, &headers).await?;
    let terms = state
        .terms
        .current()
        .ok_or_else(|| err(StatusCode::NOT_FOUND, NO_TERMS))?;
    if session.central {
        return Err(err(
            StatusCode::CONFLICT,
            "a Baylee account accepts these terms at Baylee Central",
        ));
    }
    let account_id = session.account_id;
    if body.version != terms.version {
        return Err(err(
            StatusCode::CONFLICT,
            "the terms have changed since they were shown",
        ));
    }
    store::accept_terms(&state.db, &account_id, &terms.version)
        .await
        .map_err(|e| db_down(&e))?;
    Ok(Json(TermsAccepted {
        version: terms.version.clone(),
    }))
}

/// A sign-in's `terms_stale` (login, register, guest): `true` when the
/// account last accepted another version than the current one, or none.
/// `None` on a gateway without terms, whose answers then carry no
/// `terms_stale`, so they are what they were before terms existed.
pub(crate) fn terms_stale(state: &AppState, accepted: Option<&str>) -> Option<bool> {
    state
        .terms
        .current()
        .map(|terms| accepted != Some(terms.version.as_str()))
}

/// The same for a Central-bound sign-in, from Central's own answer: whether
/// the account accepted this gateway's terms in force (`terms_stale`) and
/// Baylee Central's (`central_terms_stale`, only when it did not).
pub(crate) fn mark_standing(
    state: &AppState,
    standing: crate::terms::Standing,
    answer: &mut serde_json::Value,
) {
    if standing == crate::terms::Standing::Central {
        answer["central_terms_stale"] = true.into();
    }
    if state.terms.current().is_some() {
        answer["terms_stale"] = (standing != crate::terms::Standing::Accepted).into();
    }
}
