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
use serde::Deserialize;

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

fn listed(terms: &Terms, confirmed: bool) -> serde_json::Value {
    serde_json::json!({
        "version": terms.version,
        "digest": crate::auth::hex_lower(&terms.digest),
        "source": terms.source.as_str(),
        "at_central": confirmed,
        "texts": terms.central_texts().iter().map(|text| serde_json::json!({
            "lang": text.lang,
            "updated": text.updated,
            "markdown": text.markdown,
        })).collect::<Vec<_>>(),
    })
}

/// `GET /admin/terms`: the terms in force, where they came from and whether
/// Central holds them (`at_central`; until it does, they are shown but not
/// enforced); `{"terms": null}` for a gateway without terms.
async fn current(State(console): State<Admin>) -> Json<serde_json::Value> {
    let book = &console.app.terms;
    Json(serde_json::json!({
        "terms": book
            .current()
            .map(|terms| listed(&terms, book.confirmed().is_some())),
    }))
}

/// One language of a version an admin publishes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextBody {
    lang: String,
    #[serde(default)]
    updated: Option<String>,
    markdown: String,
}

/// What `POST /admin/terms` takes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublishBody {
    /// A new version: printable ASCII without spaces, never one published
    /// before.
    version: String,
    /// One text per language, `en` among them.
    texts: Vec<TextBody>,
}

/// `POST /admin/terms {version, texts}`: publishes a new version and puts it
/// in force. `201` with what `GET /admin/terms` answers; `400` for texts
/// outside the bounds (the sentence says which), `409` for a version
/// published before. Every player is asked again.
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
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "terms": answer })),
    )
        .into_response())
}

/// `GET /admin/terms/versions`: every version an admin published, newest
/// first: version, when, by whom.
async fn versions(State(console): State<Admin>) -> Result<Json<serde_json::Value>, Refusal> {
    let versions = baylee_db::terms::versions(&console.app.db)
        .await
        .map_err(|e| db_failed(&e))?;
    Ok(Json(serde_json::json!({
        "versions": versions.iter().map(|published| serde_json::json!({
            "version": published.version,
            "published_at": iso(published.published_at),
            "published_by": published.published_by,
        })).collect::<Vec<_>>(),
    })))
}
