//! The console's routes for who may seat which hosted model (owner,
//! 13.10.2026; `docs/protocol.md` §"Who may seat which model"), merged into
//! [`crate::admin`]'s router behind its guard. The policy itself is
//! `baylee_db::llm_access`; [`crate::seathost`] reads it where a player
//! lists or orders a model.
//!
//! A policy is per profile **id**, the id players order by over every seat
//! agent, so it may be set before any seat agent reports the profile. Every
//! change names its admin and writes an audit line with the profile id, the
//! policy and account ids; never a username or a handle.

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use baylee_db::llm_access::{self, Policy};
use baylee_protocol::seathost::valid_name;
use serde::Deserialize;
use uuid::Uuid;

use crate::admin::{Actor, Admin, Refusal, iso};
use crate::{err, err_saying};

/// The longest search the candidate lookup takes.
const MAX_SEARCH_CHARS: usize = 100;

/// The console's routes for hosted-model access.
pub(crate) fn console_routes() -> Router<Admin> {
    Router::new()
        .route("/admin/accounts/lookup", get(lookup))
        .route("/admin/llm/access", get(list))
        .route("/admin/llm/access/{id}", get(one).put(set_policy))
        .route("/admin/llm/access/{id}/allow", post(allow))
        .route("/admin/llm/access/{id}/disallow", post(disallow))
}

fn db_failed(e: &sea_orm::DbErr) -> Refusal {
    tracing::error!("admin console: hosted access: {e}");
    err(
        StatusCode::SERVICE_UNAVAILABLE,
        "the gateway's database is unavailable",
    )
}

fn profile_id(id: &str) -> Result<(), Refusal> {
    if valid_name(id) {
        Ok(())
    } else {
        Err(err(
            StatusCode::BAD_REQUEST,
            "a profile's id is 1-64 of A-Z a-z 0-9 . - _",
        ))
    }
}

/// What the seat agents report of `id` now: its label and vendor, if any
/// connected seat agent holds it.
fn reported(console: &Admin, id: &str) -> Option<(String, String)> {
    let registry = console.app.seathosts.lock();
    registry
        .rows()
        .iter()
        .find(|row| row.id == id)
        .map(|row| (row.first.label.clone(), row.first.vendor.clone()))
}

/// One profile's access as the console shows it; `stored` is `None` for a
/// profile without a row, which is listed with nobody on its list.
fn access_json(
    id: &str,
    stored: Option<&llm_access::Access>,
    seen: Option<&(String, String)>,
) -> serde_json::Value {
    let (label, vendor) = seen.cloned().unzip();
    serde_json::json!({
        "id": id,
        "policy": stored.map_or(Policy::Listed, |a| a.policy).word(),
        "stored": stored.is_some(),
        "default_for_registered": stored.is_some_and(|a| a.default_for_registered),
        "listed": stored.map_or(0, |a| a.listed),
        "updated_at": stored.map(|a| iso(a.updated_at)),
        "updated_by": stored.map(|a| a.updated_by.clone()),
        "connected": seen.is_some(),
        "label": label,
        "vendor": vendor,
    })
}

/// `GET /admin/llm/access`: every profile a seat agent reports or a policy
/// names, by id.
async fn list(State(console): State<Admin>) -> Result<Json<serde_json::Value>, Refusal> {
    let stored = llm_access::all(&console.app.db)
        .await
        .map_err(|e| db_failed(&e))?;
    let seen: std::collections::BTreeMap<String, (String, String)> = {
        let registry = console.app.seathosts.lock();
        registry
            .rows()
            .iter()
            .map(|row| {
                (
                    row.id.to_string(),
                    (row.first.label.clone(), row.first.vendor.clone()),
                )
            })
            .collect()
    };
    let mut ids: std::collections::BTreeSet<&str> = seen.keys().map(String::as_str).collect();
    ids.extend(stored.iter().map(|a| a.profile_id.as_str()));
    let profiles: Vec<serde_json::Value> = ids
        .into_iter()
        .map(|id| access_json(id, stored.iter().find(|a| a.profile_id == id), seen.get(id)))
        .collect();
    Ok(Json(serde_json::json!({ "profiles": profiles })))
}

/// One profile's access with the accounts on its list.
async fn one_json(console: &Admin, id: &str) -> Result<serde_json::Value, Refusal> {
    let db = &console.app.db;
    let stored = llm_access::one(db, id).await.map_err(|e| db_failed(&e))?;
    let allowed = llm_access::allowed(db, id)
        .await
        .map_err(|e| db_failed(&e))?;
    let mut json = access_json(id, stored.as_ref(), reported(console, id).as_ref());
    json["accounts"] = allowed
        .iter()
        .map(|a| {
            serde_json::json!({
                "id": a.account_id,
                "username": a.username,
                "display_name": a.display_name,
                "handle": crate::handle::handle(&a.display_name, a.tag),
                "added_at": iso(a.added_at),
                "added_by": a.added_by,
            })
        })
        .collect();
    Ok(json)
}

/// `GET /admin/llm/access/{id}`: one profile's policy and its list.
async fn one(
    State(console): State<Admin>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, Refusal> {
    profile_id(&id)?;
    one_json(&console, &id).await.map(Json)
}

/// What `PUT /admin/llm/access/{id}` takes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyBody {
    policy: String,
    #[serde(default)]
    default_for_registered: Option<bool>,
}

/// `PUT /admin/llm/access/{id}` `{"policy", "default_for_registered"?}`.
async fn set_policy(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, Refusal> {
    profile_id(&id)?;
    let body: PolicyBody = serde_json::from_slice(&body).map_err(|_| {
        err(
            StatusCode::BAD_REQUEST,
            "send {\"policy\": \"registered\"|\"listed\"|\"nobody\"}",
        )
    })?;
    let policy = Policy::parse(&body.policy).ok_or_else(|| {
        err(
            StatusCode::BAD_REQUEST,
            "the policy is registered, listed or nobody",
        )
    })?;
    if body.default_for_registered == Some(true) && policy != Policy::Registered {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "the default profile is open to every registered account",
        ));
    }
    let stored = llm_access::set_policy(
        &console.app.db,
        &id,
        policy,
        body.default_for_registered,
        &actor,
    )
    .await
    .map_err(|e| db_failed(&e))?;
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "llm.access.policy",
        profile = %id,
        policy = policy.word(),
        default_for_registered = stored.default_for_registered,
        "admin console: hosted model access changed"
    );
    one_json(&console, &id).await.map(Json)
}

/// What `…/allow` and `…/disallow` take.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountsBody {
    accounts: Vec<String>,
}

fn accounts_of(body: &Bytes) -> Result<Vec<Uuid>, Refusal> {
    let body: AccountsBody = serde_json::from_slice(body).map_err(|_| {
        err(
            StatusCode::BAD_REQUEST,
            "send {\"accounts\": [\"<id>\", …]}",
        )
    })?;
    if body.accounts.is_empty() || body.accounts.len() > llm_access::MAX_CHANGE {
        return Err(err_saying(
            StatusCode::BAD_REQUEST,
            format!("name 1 to {} accounts at a time", llm_access::MAX_CHANGE),
        ));
    }
    let mut ids = Vec::with_capacity(body.accounts.len());
    for id in &body.accounts {
        let id =
            Uuid::parse_str(id).map_err(|_| err(StatusCode::BAD_REQUEST, "not an account's id"))?;
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    Ok(ids)
}

fn joined(ids: &[Uuid]) -> String {
    ids.iter()
        .map(Uuid::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

/// `POST /admin/llm/access/{id}/allow` `{"accounts": [id, …]}`: puts
/// registered accounts on the profile's list. All or nothing: `404` for an
/// id no account has, `409` for a guest's.
async fn allow(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, Refusal> {
    profile_id(&id)?;
    let accounts = accounts_of(&body)?;
    let added = match llm_access::allow(&console.app.db, &id, &accounts, &actor)
        .await
        .map_err(|e| db_failed(&e))?
    {
        Ok(added) => added,
        Err(llm_access::Refused::Unknown(account)) => {
            return Err(err_saying(
                StatusCode::NOT_FOUND,
                format!("no account {account}"),
            ));
        }
        Err(llm_access::Refused::Guest(account)) => {
            return Err(err_saying(
                StatusCode::CONFLICT,
                format!("account {account} is a guest's; a guest cannot seat a hosted model"),
            ));
        }
    };
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "llm.access.allow",
        profile = %id,
        accounts = %joined(&accounts),
        added,
        "admin console: hosted model access changed"
    );
    let mut json = one_json(&console, &id).await?;
    json["added"] = added.into();
    Ok(Json(json))
}

/// `POST /admin/llm/access/{id}/disallow` `{"accounts": [id, …]}`: takes
/// accounts off the profile's list; an id not on it is no error.
async fn disallow(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<serde_json::Value>, Refusal> {
    profile_id(&id)?;
    let accounts = accounts_of(&body)?;
    let removed = llm_access::disallow(&console.app.db, &id, &accounts)
        .await
        .map_err(|e| db_failed(&e))?;
    tracing::info!(
        target: "baylee_gateway::audit",
        admin = %actor,
        action = "llm.access.disallow",
        profile = %id,
        accounts = %joined(&accounts),
        removed,
        "admin console: hosted model access changed"
    );
    let mut json = one_json(&console, &id).await?;
    json["removed"] = removed.into();
    Ok(Json(json))
}

/// What `GET /admin/accounts/lookup` takes.
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct LookupQuery {
    q: String,
}

/// `GET /admin/accounts/lookup?q=`: registered accounts whose username or
/// display name starts with `q`, for the allow-list editor's type-ahead;
/// fewer than two characters find nothing, at most ten are answered. A
/// read, so not audited.
async fn lookup(
    State(console): State<Admin>,
    query: Result<Query<LookupQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<serde_json::Value>, Refusal> {
    let Query(query) = query.map_err(|_| err(StatusCode::BAD_REQUEST, "send ?q=<prefix>"))?;
    if query.q.chars().count() > MAX_SEARCH_CHARS {
        return Err(err(StatusCode::BAD_REQUEST, "the search is too long"));
    }
    let found = llm_access::candidates(&console.app.db, &query.q, llm_access::MAX_CANDIDATES)
        .await
        .map_err(|e| db_failed(&e))?;
    let accounts: Vec<serde_json::Value> = found
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.account_id,
                "username": c.username,
                "display_name": c.display_name,
                "handle": crate::handle::handle(&c.display_name, c.tag),
            })
        })
        .collect();
    Ok(Json(serde_json::json!({ "accounts": accounts })))
}
