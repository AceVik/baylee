//! The console's routes for the gateway's terms of use (`docs/protocol.md`
//! §"Terms of use (WG-1)"), merged into [`crate::admin`]'s router behind its
//! guard. An admin reads what is in force and publishes a new version; a
//! published version never changes, the newest is in force, and it goes to
//! Baylee Central at once so players can accept it there. Every publish
//! names its admin in an audit line, never the text.

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use serde::{Deserialize, Serialize};

use crate::admin::{Actor, Admin, Refusal, iso};
use crate::terms::Terms;
use crate::{err, err_saying};

/// The console's routes for the terms of use.
pub(crate) fn console_routes() -> Router<Admin> {
    Router::new()
        .route("/admin/terms", get(current).post(publish))
        .route("/admin/terms/versions", get(versions))
}

fn db_failed(e: &sea_orm::DbErr) -> Refusal {
    tracing::error!("admin console: terms: {e}");
    err(
        StatusCode::SERVICE_UNAVAILABLE,
        "the gateway's database is unavailable",
    )
}

/// `GET /admin/terms` and `POST /admin/terms`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct TermsInForce {
    /// @description The terms in force, or `null` for a gateway without
    /// terms.
    terms: Option<ConsoleTerms>,
}

/// The terms in force, as the console shows them.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleTerms {
    /// @description What an account accepts.
    version: String,
    /// @description Baylee Central's digest of every text, in lowercase
    /// hex: what an acceptance names beside the version.
    digest: String,
    /// @description Where they came from: an admin published them, a
    /// file (`BAYLEE_TERMS_PATH`) named them, or they are the default every
    /// gateway ships.
    /// @openapi enum: [published, file, default]
    source: &'static str,
    /// @description Whether Baylee Central holds them; until it does, they
    /// are shown and not enforced.
    at_central: bool,
    /// @description One text per language (`en` for a single file).
    texts: Vec<ConsoleTermsText>,
}

/// The terms in one language.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ConsoleTermsText {
    /// @description Its primary language tag, lowercase (`de`, `en`).
    lang: String,
    /// @description The date it names as its last change, or `null`.
    updated: Option<String>,
    /// @description The text, in Markdown.
    markdown: String,
}

fn listed(terms: &Terms, confirmed: bool) -> ConsoleTerms {
    ConsoleTerms {
        version: terms.version.clone(),
        digest: crate::auth::hex_lower(&terms.digest),
        source: terms.source.as_str(),
        at_central: confirmed,
        texts: terms
            .central_texts()
            .into_iter()
            .map(|text| ConsoleTermsText {
                lang: text.lang,
                updated: text.updated,
                markdown: text.markdown,
            })
            .collect(),
    }
}

/// `GET /admin/terms`: the terms in force, where they came from and whether
/// Central holds them (`at_central`; until it does, they are shown but not
/// enforced); `{"terms": null}` for a gateway without terms.
///
/// @route GET /admin/terms
/// @scope admin
/// @summary The terms of use in force
/// @description The terms in force, where they came from, and whether
/// Baylee Central holds them (until it does, they are shown and not
/// enforced); `{"terms": null}` for a gateway without terms.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "Baylee Central or the gateway's database is unavailable while an admin lease is checked"
async fn current(State(console): State<Admin>) -> Json<TermsInForce> {
    let book = &console.app.terms;
    Json(TermsInForce {
        terms: book
            .current()
            .map(|terms| listed(&terms, book.confirmed().is_some())),
    })
}

/// One language of a version an admin publishes.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextBody {
    /// @description A lowercase primary language tag (`de`, `en`); each
    /// at most once.
    lang: String,
    /// @description The date the text names as its last change; absent,
    /// `null` or empty for none.
    #[serde(default)]
    updated: Option<String>,
    /// @description The text, in Markdown: at most 64 KiB, no control
    /// characters but newline and tab.
    markdown: String,
}

/// What `POST /admin/terms` takes.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublishBody {
    /// @description A new version: printable ASCII without spaces, at most
    /// 64 characters, not `draft`, and never one published before or the
    /// one in force.
    /// @internal
    /// A new version: printable ASCII without spaces, never one published
    /// before.
    version: String,
    /// @description One text per language, at most 4, `en` among them.
    /// @internal
    /// One text per language, `en` among them.
    texts: Vec<TextBody>,
}

/// `POST /admin/terms {version, texts}`: publishes a new version and puts it
/// in force. `201` with what `GET /admin/terms` answers; `400` for texts
/// outside the bounds (the sentence says which), `409` for a version
/// published before. Every player is asked again.
///
/// @route POST /admin/terms
/// @scope admin
/// @summary Publish a new version of the terms of use
/// @description Publishes a new version and puts it in force; it goes to
/// Baylee Central at once, and every player is asked to accept it. A
/// published version never changes: a changed text needs a new version.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line and the version's `published_by`; ignored when the bearer is a Central admin lease, whose account acts"
/// @body `$PublishBody`
/// @return 201: `$TermsInForce` "Published and in force, as `GET /admin/terms` answers"
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, not terms to publish (an unknown field included), or texts or a version outside the bounds; the sentence says which"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 409: `$ErrorBody` "The version was published before, or is the one in force"
/// @return 413: "The body is over 16 KiB"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn publish(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    body: Bytes,
) -> Result<Response, Refusal> {
    let body: PublishBody = serde_json::from_slice(&body)
        .map_err(|_| err(StatusCode::BAD_REQUEST, "not terms of use to publish"))?;
    let terms = Terms::published(
        body.version,
        body.texts
            .into_iter()
            .map(|text| (text.lang, text.updated, text.markdown))
            .collect(),
    )
    .map_err(|why| err_saying(StatusCode::BAD_REQUEST, why))?;
    // The version in force, from the file or the default, is not in the
    // database, but Central holds it already: another text under it would
    // never be confirmed there, so it is refused here.
    if console
        .app
        .terms
        .current()
        .is_some_and(|current| current.version == terms.version)
    {
        return Err(err_saying(
            StatusCode::CONFLICT,
            format!(
                "version {} is in force already; publish the new text under a new version",
                terms.version
            ),
        ));
    }
    let texts: Vec<baylee_db::terms::TermsText> = terms
        .central_texts()
        .into_iter()
        .map(|text| baylee_db::terms::TermsText {
            lang: text.lang,
            updated: text.updated,
            markdown: text.markdown,
        })
        .collect();
    let stored = baylee_db::terms::publish(
        &console.app.db,
        &terms.version,
        &terms.digest,
        &texts,
        &actor,
    )
    .await
    .map_err(|e| db_failed(&e))?;
    if !stored {
        return Err(err_saying(
            StatusCode::CONFLICT,
            format!(
                "version {} was published before; a published version never changes, so \
                 publish the new text under a new version",
                terms.version
            ),
        ));
    }
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "terms.publish",
        version = %terms.version,
        "admin console: terms of use published"
    );
    console.app.terms.replace(terms);
    let at_central = crate::central::publish_terms(&console.app).await;
    let book = &console.app.terms;
    let answer = book
        .current()
        .map(|terms| listed(&terms, at_central && book.confirmed().is_some()));
    Ok((StatusCode::CREATED, Json(TermsInForce { terms: answer })).into_response())
}

/// `GET /admin/terms/versions`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct TermsVersions {
    /// @description Every version an admin published, newest first.
    versions: Vec<PublishedTerms>,
}

/// A version of the terms an admin published.
/// @openapi
#[derive(Serialize)]
pub(crate) struct PublishedTerms {
    /// @description The version.
    version: String,
    /// @description When it was published, RFC 3339 in UTC.
    /// @openapi format: date-time
    published_at: String,
    /// @description The admin who published it.
    published_by: String,
}

/// `GET /admin/terms/versions`: every version an admin published, newest
/// first: version, when, by whom.
///
/// @route GET /admin/terms/versions
/// @scope admin
/// @summary Every version of the terms an admin published
/// @description Every version an admin published, newest first, with when
/// and by whom; versions from a file or the default are not listed.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn versions(State(console): State<Admin>) -> Result<Json<TermsVersions>, Refusal> {
    let versions = baylee_db::terms::versions(&console.app.db)
        .await
        .map_err(|e| db_failed(&e))?;
    Ok(Json(TermsVersions {
        versions: versions
            .iter()
            .map(|published| PublishedTerms {
                version: published.version.clone(),
                published_at: iso(published.published_at),
                published_by: published.published_by.clone(),
            })
            .collect(),
    }))
}
