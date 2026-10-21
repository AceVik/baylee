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
use serde::{Deserialize, Serialize};
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

/// Who may seat one hosted profile.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ProfileAccess {
    /// @description The profile's id, over every seat agent.
    id: String,
    /// @description Who may seat it: every registered account, only the
    /// accounts on its list, or nobody. A guest may seat none.
    /// @openapi enum: [registered, listed, nobody]
    policy: &'static str,
    /// @description Whether a policy is kept for it; `false` reads as
    /// `listed` with nobody on the list.
    stored: bool,
    /// @description Whether it is the operator's default seat for every
    /// registered account.
    default_for_registered: bool,
    /// @description How many accounts are on its list.
    listed: i64,
    /// @description When its policy last changed, RFC 3339 in UTC, or
    /// `null`.
    /// @openapi format: date-time
    updated_at: Option<String>,
    /// @description The admin who changed it, or `null`.
    updated_by: Option<String>,
    /// @description Whether a seat agent reports it now.
    connected: bool,
    /// @description What a seat agent reports it called, or `null`.
    label: Option<String>,
    /// @description Who a seat agent reports the game data goes to, or
    /// `null`.
    vendor: Option<String>,
}

/// One profile's access as the console shows it; `stored` is `None` for a
/// profile without a row, which is listed with nobody on its list.
fn access_json(
    id: &str,
    stored: Option<&llm_access::Access>,
    seen: Option<&(String, String)>,
) -> ProfileAccess {
    let (label, vendor) = seen.cloned().unzip();
    ProfileAccess {
        id: id.to_owned(),
        policy: stored.map_or(Policy::Listed, |a| a.policy).word(),
        stored: stored.is_some(),
        default_for_registered: stored.is_some_and(|a| a.default_for_registered),
        listed: stored.map_or(0, |a| a.listed),
        updated_at: stored.map(|a| iso(a.updated_at)),
        updated_by: stored.map(|a| a.updated_by.clone()),
        connected: seen.is_some(),
        label,
        vendor,
    }
}

/// `GET /admin/llm/access`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ProfileAccessList {
    /// @description Every profile a seat agent reports or a policy names,
    /// by id.
    profiles: Vec<ProfileAccess>,
}

/// `GET /admin/llm/access`: every profile a seat agent reports or a policy
/// names, by id.
///
/// @route GET /admin/llm/access
/// @scope admin
/// @summary Who may seat each hosted profile
/// @description Every profile a seat agent reports or a policy names, by
/// id, with its policy and whether a seat agent reports it now. A profile
/// without a policy is open to nobody.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn list(State(console): State<Admin>) -> Result<Json<ProfileAccessList>, Refusal> {
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
    let profiles: Vec<ProfileAccess> = ids
        .into_iter()
        .map(|id| access_json(id, stored.iter().find(|a| a.profile_id == id), seen.get(id)))
        .collect();
    Ok(Json(ProfileAccessList { profiles }))
}

/// One profile's access with the accounts on its list.
/// @openapi
#[derive(Serialize)]
pub(crate) struct ProfileAccessDetail {
    #[serde(flatten)]
    access: ProfileAccess,
    /// @description The accounts on its list, by username.
    accounts: Vec<AllowedAccount>,
}

/// An account on a profile's list.
/// @openapi
#[derive(Serialize)]
pub(crate) struct AllowedAccount {
    /// @description The account's id.
    id: Uuid,
    /// @description Its username, or `null`.
    username: Option<String>,
    /// @description What other players see.
    display_name: String,
    /// @description Its handle, `Name#tag`.
    handle: String,
    /// @description When it was put on the list, RFC 3339 in UTC.
    /// @openapi format: date-time
    added_at: String,
    /// @description The admin who put it there.
    added_by: String,
}

/// One profile's access with the accounts on its list.
async fn one_json(console: &Admin, id: &str) -> Result<ProfileAccessDetail, Refusal> {
    let db = &console.app.db;
    let stored = llm_access::one(db, id).await.map_err(|e| db_failed(&e))?;
    let allowed = llm_access::allowed(db, id)
        .await
        .map_err(|e| db_failed(&e))?;
    Ok(ProfileAccessDetail {
        access: access_json(id, stored.as_ref(), reported(console, id).as_ref()),
        accounts: allowed
            .iter()
            .map(|a| AllowedAccount {
                id: a.account_id,
                username: a.username.clone(),
                display_name: a.display_name.clone(),
                handle: crate::handle::handle(&a.display_name, a.tag),
                added_at: iso(a.added_at),
                added_by: a.added_by.clone(),
            })
            .collect(),
    })
}

/// `GET /admin/llm/access/{id}`: one profile's policy and its list.
///
/// @route GET /admin/llm/access/{id}
/// @scope admin
/// @summary Who may seat one hosted profile
/// @description One profile's policy and the accounts on its list. A
/// profile no seat agent reports and no policy names reads as `listed`
/// with nobody on it.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @path-param id: String "The profile's id: 1-64 of A-Z a-z 0-9 . - _"
/// @return 400: `$ErrorBody` "The id is not 1-64 of A-Z a-z 0-9 . - _"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn one(
    State(console): State<Admin>,
    Path(id): Path<String>,
) -> Result<Json<ProfileAccessDetail>, Refusal> {
    profile_id(&id)?;
    one_json(&console, &id).await.map(Json)
}

/// What `PUT /admin/llm/access/{id}` takes.
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyBody {
    /// @description Who may seat the profile.
    /// @openapi enum: [registered, listed, nobody]
    policy: String,
    /// @description `true` makes it the operator's default seat, taking the
    /// mark from any other profile (only with `registered`); `false` takes
    /// the mark away; absent leaves it, though another policy than
    /// `registered` takes it away too.
    #[serde(default)]
    default_for_registered: Option<bool>,
}

/// `PUT /admin/llm/access/{id}` `{"policy", "default_for_registered"?}`.
///
/// @route PUT /admin/llm/access/{id}
/// @scope admin
/// @summary Set who may seat a hosted profile
/// @description Sets the profile's policy, and whether it is the default
/// seat; answers as `GET`. A change stops nothing already ordered: it is
/// read at each listing and each order.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line and `updated_by`; ignored when the bearer is a Central admin lease, whose account acts"
/// @path-param id: String "The profile's id: 1-64 of A-Z a-z 0-9 . - _"
/// @body `$PolicyBody`
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, an id outside 1-64 of A-Z a-z 0-9 . - _, not a policy (an unknown field included), another policy word, or the default mark with a policy other than `registered`"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 413: "The body is over 16 KiB"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn set_policy(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<ProfileAccessDetail>, Refusal> {
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
/// @openapi
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AccountsBody {
    /// @description Account ids, 1 to 200; an id named twice counts once.
    /// @openapi
    /// ```yaml
    /// minItems: 1
    /// ```
    /// @openapi
    /// ```yaml
    /// maxItems: 200
    /// ```
    /// @openapi items: { type: string, format: uuid }
    accounts: Vec<String>,
}

/// `POST /admin/llm/access/{id}/allow`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct AccessAdded {
    #[serde(flatten)]
    detail: ProfileAccessDetail,
    /// @description How many of the accounts were not on the list already.
    added: u64,
}

/// `POST /admin/llm/access/{id}/disallow`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct AccessRemoved {
    #[serde(flatten)]
    detail: ProfileAccessDetail,
    /// @description How many of the accounts were on the list.
    removed: u64,
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
///
/// @route POST /admin/llm/access/{id}/allow
/// @scope admin
/// @summary Put accounts on a hosted profile's list
/// @description Puts registered accounts on the profile's list, all or
/// nothing; a profile without a policy gets `listed`. Answers as `GET`,
/// with how many were added.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line and `added_by`; ignored when the bearer is a Central admin lease, whose account acts"
/// @path-param id: String "The profile's id: 1-64 of A-Z a-z 0-9 . - _"
/// @body `$AccountsBody`
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, an id outside 1-64 of A-Z a-z 0-9 . - _, not a list of accounts (an unknown field included), fewer than 1 or more than 200, or one that is not a UUID"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 404: `$ErrorBody` "An id no account has; nobody was added"
/// @return 409: `$ErrorBody` "A guest's account, which cannot seat a hosted model; nobody was added"
/// @return 413: "The body is over 16 KiB"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn allow(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<AccessAdded>, Refusal> {
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
    Ok(Json(AccessAdded {
        detail: one_json(&console, &id).await?,
        added,
    }))
}

/// `POST /admin/llm/access/{id}/disallow` `{"accounts": [id, …]}`: takes
/// accounts off the profile's list; an id not on it is no error.
///
/// @route POST /admin/llm/access/{id}/disallow
/// @scope admin
/// @summary Take accounts off a hosted profile's list
/// @description Takes accounts off the profile's list; an id not on it is
/// no error. Answers as `GET`, with how many were removed.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @header-param X-Baylee-Admin: String required `pattern=^[\w.-]{1,64}$` "The acting admin's name, for the audit line; ignored when the bearer is a Central admin lease, whose account acts"
/// @path-param id: String "The profile's id: 1-64 of A-Z a-z 0-9 . - _"
/// @body `$AccountsBody`
/// @return 400: `$ErrorBody` "No valid `X-Baylee-Admin`, an id outside 1-64 of A-Z a-z 0-9 . - _, not a list of accounts (an unknown field included), fewer than 1 or more than 200, or one that is not a UUID"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 413: "The body is over 16 KiB"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn disallow(
    State(console): State<Admin>,
    axum::Extension(Actor(actor)): axum::Extension<Actor>,
    Path(id): Path<String>,
    body: Bytes,
) -> Result<Json<AccessRemoved>, Refusal> {
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
    Ok(Json(AccessRemoved {
        detail: one_json(&console, &id).await?,
        removed,
    }))
}

/// What `GET /admin/accounts/lookup` takes.
/// @openapi
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct LookupQuery {
    /// @description The start of a username or display name, any case
    /// (`%` and `_` are themselves); fewer than 2 characters find nothing,
    /// at most 100.
    q: String,
}

/// `GET /admin/accounts/lookup`.
/// @openapi
#[derive(Serialize)]
pub(crate) struct LookupAnswer {
    /// @description At most 10 registered accounts, the username equal to
    /// `q` first, then by username.
    accounts: Vec<LookedUp>,
}

/// An account the type-ahead found.
/// @openapi
#[derive(Serialize)]
pub(crate) struct LookedUp {
    /// @description The account's id.
    id: Uuid,
    /// @description Its username, or `null`.
    username: Option<String>,
    /// @description What other players see.
    display_name: String,
    /// @description Its handle, `Name#tag`.
    handle: String,
}

/// `GET /admin/accounts/lookup?q=`: registered accounts whose username or
/// display name starts with `q`, for the allow-list editor's type-ahead;
/// fewer than two characters find nothing, at most ten are answered. A
/// read, so not audited.
///
/// @route GET /admin/accounts/lookup
/// @scope admin
/// @summary Find registered accounts by the start of a name
/// @description Registered accounts whose username or display name starts
/// with `q`, for a list editor's type-ahead: the username equal to `q`
/// first, then by username, at most 10.
/// @tag Admin
/// @security `adminToken`
/// @server <http://127.0.0.1:28767> "The admin listener (loopback only)"
/// @query-params `$LookupQuery`
/// @return 400: `$ErrorBody` "A query parameter other than `q`, or a search over 100 characters"
/// @return 401: `$ErrorBody` "The bearer token is neither `BAYLEE_ADMIN_TOKEN` nor a live Central admin lease"
/// @return 403: `$ErrorBody` "A browser's request (`Origin`, `Sec-Fetch-Site` or `Sec-Fetch-Mode`) or a `Host` that is not this machine, refused before the token is read"
/// @return 429: `$ErrorBody` "Ten wrong tokens in five minutes, or 600 requests in a minute; the right token is refused too until the window passes"
/// @return-header 429 Retry-After: u64 "Seconds to wait: 300"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable (or Baylee Central, while an admin lease is checked)"
async fn lookup(
    State(console): State<Admin>,
    query: Result<Query<LookupQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<LookupAnswer>, Refusal> {
    let Query(query) = query.map_err(|_| err(StatusCode::BAD_REQUEST, "send ?q=<prefix>"))?;
    if query.q.chars().count() > MAX_SEARCH_CHARS {
        return Err(err(StatusCode::BAD_REQUEST, "the search is too long"));
    }
    let found = llm_access::candidates(&console.app.db, &query.q, llm_access::MAX_CANDIDATES)
        .await
        .map_err(|e| db_failed(&e))?;
    let accounts: Vec<LookedUp> = found
        .iter()
        .map(|c| LookedUp {
            id: c.account_id,
            username: c.username.clone(),
            display_name: c.display_name.clone(),
            handle: crate::handle::handle(&c.display_name, c.tag),
        })
        .collect();
    Ok(Json(LookupAnswer { accounts }))
}
