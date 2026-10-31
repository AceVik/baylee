//! Accounts from the outside: guests, signing out, and what an account may
//! read about itself and others. Nobody registers or signs in with a
//! password here: accounts are Baylee Central's (`central::routes`), and a
//! gateway connected to Central takes no guests (`docs/central-login.md`).

use crate::{
    AppState, ConnectInfo, Deserialize, ErrorBody, HeaderMap, Json, Path, Router, Serialize,
    Shared, SocketAddr, State, StatusCode, account, auth, auth_config, authed, db_down, delete,
    err, get, get_settings, handle, invite, post, put_settings, rate_limit_ip, store,
};

use super::terms::{accept_terms, terms, terms_stale};

/// The closed-beta key a new guest brings (#317), as the hash
/// its row is found by; `None` on a gateway that asks for none.
///
/// A request that brings no key is refused in words a client from before
/// keys can show ([`invite::KEY_NEEDED`]). Every key brought is counted
/// against the address it came from, on the sign-in limiter, before
/// anything else is done with it: eight tries in five minutes, whether they
/// were well formed or not, so that guessing at eighty bits stays a guess.
/// A key that is not one is refused as a key that admits nobody is, in one
/// sentence ([`invite::KEY_INVALID`]). The key itself is never logged.
pub(crate) fn admission(
    state: &AppState,
    ip: &str,
    offered: Option<&str>,
) -> Result<Option<Vec<u8>>, (StatusCode, Json<ErrorBody>)> {
    if state.registration != invite::Registration::Invite {
        return Ok(None);
    }
    let Some(offered) = offered.filter(|key| !key.trim().is_empty()) else {
        return Err(err(StatusCode::FORBIDDEN, invite::KEY_NEEDED));
    };
    if !state.sign_in_limiter.allow(&invite_budget(ip)) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    invite::canonical(offered)
        .map(|key| Some(invite::digest(&key)))
        .ok_or_else(|| err(StatusCode::FORBIDDEN, invite::KEY_INVALID))
}

/// A key admitted somebody: the tries from that address start over.
pub(crate) fn admitted(state: &AppState, ip: &str, key: Option<&[u8]>) {
    if key.is_some() {
        state.sign_in_limiter.forget(&invite_budget(ip));
    }
}

/// What the sign-in limiter counts an address's key tries under.
pub(crate) fn invite_budget(ip: &str) -> String {
    format!("invite:{ip}")
}

/// What `POST /auth/guest` takes (#269). Every field is optional, and an
/// empty body is a guest called `Guest`.
///
/// @description What `POST /auth/guest` takes. Every field is optional:
/// `{}` is a guest called `Guest`.
/// @openapi
#[derive(Deserialize, Default)]
pub(crate) struct GuestBody {
    /// @description The name other players see, under the display-name
    /// rule; `Guest` when absent or empty.
    /// @internal
    /// The name other players see, under the display-name rule.
    #[serde(default)]
    display_name: Option<String>,
    /// @description The language it asks in (`de`, `en`).
    /// @internal
    /// The language it asks in.
    #[serde(default)]
    lang: String,
    /// @description The closed-beta key a new guest needs where
    /// registration is `invite`; ignored elsewhere.
    /// @internal
    /// The closed-beta key a new guest needs on a gateway with
    /// `BAYLEE_REGISTRATION=invite` (#317).
    #[serde(default)]
    invite_key: Option<String>,
}

/// What `POST /auth/guest` answers.
/// @openapi
#[derive(Serialize)]
pub(crate) struct GuestSignedIn {
    /// @description The session token, sent as `Authorization: Bearer`: the
    /// only way back to this guest, so the client keeps it.
    token: String,
    /// @description When the session lapses unless it is used before, in
    /// Unix seconds; a guest whose session lapsed is deleted.
    expires_at: u64,
    /// @description Always `true`.
    guest: bool,
    /// @description The guest's handle as other players see it, its display
    /// name and tag (`Guest#1a2b`).
    handle: String,
    /// @description Always `true` for a new guest, which has accepted no
    /// terms yet; absent on a gateway without terms.
    #[serde(skip_serializing_if = "Option::is_none")]
    terms_stale: Option<bool>,
}

/// The name a guest is seen by when it chose none.
pub(crate) const GUEST_NAME: &str = "Guest";

/// `POST /auth/guest`: an account with no username, no address and no
/// password, and its session, in one answer (#269).
///
/// The session is the guest: nothing signs in as one, so it lives as long
/// as its session does ([`auth::Lifetime::GUEST`], 29 to 30 days from the
/// last call made with its token) and goes with it, decks included
/// ([`store::purge_guests`]). Bounded twice: per address by the gateway's limiter, and in all
/// by [`AppState::guest_cap`]. The count is read before the write, so a
/// burst of requests at the edge can pass it by as many as are in flight;
/// the cap bounds a flood, not a queue.
///
/// @route POST /auth/guest
/// @scope public
/// @summary Play as a guest
/// @description An account with no username, address or password, and its
/// session, in one answer: the session is the guest. It lasts 29 to 30 days
/// past the last call made with its token, and the guest is deleted with
/// its decks and preferences when it lapses or signs out. A guest may not
/// upload pictures. On a closed beta a new guest needs a key.
/// @tag Accounts
/// @security none
/// @return 400: `$ErrorBody` "The display name breaks its rule"
/// @return 403: `$ErrorBody` "This gateway takes no guests (its mode is `central`), or a closed beta's key is missing or not valid"
/// @return 429: `$ErrorBody` "Too many attempts from this address, or too many keys tried"
/// @return 503: `$ErrorBody` "No guest seats are free (`BAYLEE_GUEST_CAP`), or the gateway's database is unavailable"
pub(crate) async fn guest(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<GuestBody>,
) -> Result<Json<GuestSignedIn>, (StatusCode, Json<ErrorBody>)> {
    // The mode in force decides (`authmode.rs`, `docs/central-login.md`
    // §"Who a gateway lets in").
    if !state.auth_mode.current().guests() {
        return Err(err(StatusCode::FORBIDDEN, "this gateway takes no guests"));
    }
    let ip = rate_limit_ip(&state.trusted_proxies, addr.ip(), &headers);
    if !state.limiter.allow(&ip) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    let display_name = body
        .display_name
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| GUEST_NAME.to_owned());
    if !auth::valid_display_name(&display_name) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid display name"));
    }
    // A closed beta's guest door is a door too (#317). A guest this device
    // keeps comes back with its session and never asks this route again.
    let key = admission(&state, &ip, body.invite_key.as_deref())?;
    if let Some(cap) = state.guest_cap {
        let guests = store::guest_count(&state.db)
            .await
            .map_err(|e| db_down(&e))?;
        if guests >= cap {
            return Err(err(
                StatusCode::SERVICE_UNAVAILABLE,
                "no guest seats free, sign up or try later",
            ));
        }
    }
    let issued = auth::IssuedToken::lasting(auth::Lifetime::GUEST);
    let Some(account) = store::create_guest(
        &state.db,
        store::NewGuest {
            display_name,
            created_at: auth::now_secs(),
            lang: body.lang,
        },
        auth::token_digest(&issued.token),
        issued.expires_at,
        key.as_deref(),
    )
    .await
    .map_err(|e| db_down(&e))?
    else {
        return Err(err(StatusCode::FORBIDDEN, invite::KEY_INVALID));
    };
    admitted(&state, &ip, key.as_deref());
    Ok(Json(GuestSignedIn {
        token: issued.token,
        expires_at: issued.expires_at,
        guest: true,
        handle: handle::handle(&account.display_name, account.tag),
        // A new guest has accepted nothing yet.
        terms_stale: terms_stale(&state, None),
    }))
}

/// `POST /auth/logout`: ends the session the bearer token names.
///
/// @route POST /auth/logout
/// @scope public
/// @summary Sign out
/// @description Ends the session the bearer token names; a token that names
/// no session is answered the same. A guest's session is the guest, so a
/// guest signing out is deleted with its decks and preferences. Open to a
/// session that has not accepted the terms in force.
/// @tag Accounts
/// @security bearer
/// @return 204: "Signed out"
/// @return 401: `$ErrorBody` "No bearer token"
/// @return 503: `$ErrorBody` "The gateway's database is unavailable"
pub(crate) async fn logout(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    let token = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "missing bearer token"))?;
    let owner = store::drop_token(&state.db, token)
        .await
        .map_err(|e| db_down(&e))?;
    // A guest's session is the guest (#269): signed out, nothing can reach
    // it again, so it goes now rather than at the next sweep. The client
    // asked the player first. An account's is left alone: the query deletes
    // guests only.
    if let Some(account_id) = owner {
        let now = auth::now_secs();
        account::depart(
            &state,
            store::purge_guests(&state.db, now, Some(&account_id)),
        )
        .await
        .map_err(|e| db_down(&e))?;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Signing in and out, and the account itself.
///
/// Split off the router `main` builds as [`deck_routes`] is, when deleting
/// an account (#292) made it one route too long for its function.
pub(crate) fn account_routes() -> Router<Shared> {
    Router::new()
        .route("/auth/config", get(auth_config))
        .route("/auth/guest", post(guest))
        .route("/auth/logout", post(logout))
        .route("/account", delete(account::delete_account))
        .route("/me", get(me))
        .route("/players/{handle}", get(player))
        .route("/settings", get(get_settings).put(put_settings))
        .route("/terms", get(terms))
        .route("/account/terms", post(accept_terms))
}

/// The authenticated account's profile.
///
/// @route GET /me
/// @scope public
/// @summary Who am I
/// @description The signed-in account as its owner sees it: its id, its
/// address and username (shown to nobody else), whether it is a guest, and
/// its display name, tag and handle.
/// @tag Accounts
/// @security bearer
/// @insert `SessionRefusals`
/// @return 404: `$ErrorBody` "The account is gone"
pub(crate) async fn me(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<Profile>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let account = store::account(&state.db, &account_id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "account gone"))?;
    Ok(Json(Profile {
        handle: handle::handle(&account.display_name, account.tag),
        id: account.id,
        email: account.email,
        // Only ever to its owner: nothing that shows one player to another
        // carries it (#269).
        username: account.username,
        // A guest's client says what a guest is (#269): gone about thirty
        // days after its last visit.
        guest: account.guest,
        display_name: account.display_name,
        // The two halves separately, because this is the one caller that
        // wants them apart: a settings screen shows the name in a field a
        // player can edit and the tag beside it as something they cannot.
        tag: handle::tag_text(account.tag),
    }))
}

/// What `GET /me` answers.
/// @openapi
#[derive(Serialize)]
pub(crate) struct Profile {
    /// @description The account's id (a UUID).
    id: String,
    /// @description The address it registered with before usernames, `null`
    /// when it has none.
    email: Option<String>,
    /// @description The name it signs in with, shown to nobody but its
    /// owner; `null` for a guest.
    username: Option<String>,
    /// @description Whether it is a guest, deleted about thirty days after
    /// its last visit.
    guest: bool,
    /// @description The name other players see.
    display_name: String,
    /// @description The tag that tells two of a name apart: lowercase hex,
    /// at least four digits (`af03`).
    tag: String,
    /// @description The display name and tag as other players see them
    /// (`Alice#af03`).
    handle: String,
}

/// The account a handle names.
///
/// `GET /players/Alice%23af03`, or `GET /players/%23af03` when all that was
/// pasted was the tag. What comes back is what one player may know about
/// another: the id, the name and the tag — never the address.
///
/// Behind a session, because the tags are sequential and walking them would
/// otherwise list every account on the gateway to anyone who asked. A signed
/// -in player can still walk them, which is the cost of a tag a person can
/// type; it is the same cost a `BattleTag` has, and the reason this answers
/// nothing an opponent could not read off a lobby row.
///
/// @route GET /players/{handle}
/// @scope public
/// @summary Find a player by handle
/// @description What one player may know about another, found by handle:
/// the id, display name, tag and handle, never an address or username. The
/// tag decides; a handle without `#` is refused rather than guessed at.
/// @tag Accounts
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param handle: String "A handle, `Alice#af03`, or only its tag, `#af03`; the `#` URL-encoded as `%23`"
/// @return 400: `$ErrorBody` "The handle carries no `#tag`"
/// @return 404: `$ErrorBody` "No player has that tag"
pub(crate) async fn player(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(typed): Path<String>,
) -> Result<Json<PublicPlayer>, (StatusCode, Json<ErrorBody>)> {
    let _ = authed(&state, &headers).await?;
    let tag = handle::parse_tag(&typed)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "a handle carries a #tag"))?;
    let found = store::account_by_tag(&state.db, tag)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such player"))?;
    Ok(Json(PublicPlayer {
        handle: handle::handle(&found.display_name, found.tag),
        tag: handle::tag_text(found.tag),
        id: found.id,
        display_name: found.display_name,
    }))
}

/// What `GET /players/{handle}` answers.
/// @openapi
#[derive(Serialize)]
pub(crate) struct PublicPlayer {
    /// @description The account's id (a UUID).
    id: String,
    /// @description The name other players see.
    display_name: String,
    /// @description The tag that tells two of a name apart: lowercase hex,
    /// at least four digits (`af03`).
    tag: String,
    /// @description The display name and tag (`Alice#af03`).
    handle: String,
}
