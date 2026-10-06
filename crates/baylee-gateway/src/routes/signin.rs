//! Accounts from the outside: registering, confirming an address, signing
//! in and out, guests, and what an account may read about itself and others.

use crate::{
    AppState, Confirmation, ConnectInfo, Deserialize, ErrorBody, HeaderMap, Json, Path, Query,
    Router, Shared, SocketAddr, State, StatusCode, StoredToken, account, auth, auth_config, authed,
    db_down, delete, err, get, get_settings, handle, invite, names, post, put_settings,
    rate_limit_ip, store,
};

#[derive(Deserialize)]
pub(crate) struct RegisterBody {
    /// The name to sign in with (#269). Defaulted so that a client from
    /// before usernames, which sends an address instead, is told what is
    /// missing rather than handed a deserialiser's error.
    #[serde(default)]
    username: String,
    display_name: String,
    password: String,
    /// The language to write to this account in. Defaulted, because a client
    /// written before this field existed still registers.
    #[serde(default)]
    lang: String,
    /// The closed-beta key, as typed (#317). Read only on a gateway with
    /// `BAYLEE_REGISTRATION=invite`, ignored elsewhere.
    #[serde(default)]
    invite_key: Option<String>,
}

/// What `POST /auth/confirm/resend` takes.
#[derive(Deserialize)]
pub(crate) struct ResendBody {
    email: String,
}

/// What `GET /auth/confirm` takes.
#[derive(Deserialize)]
pub(crate) struct ConfirmQuery {
    token: String,
}

#[derive(Deserialize)]
pub(crate) struct Credentials {
    /// A username, or until the end of 2026 an address (#269, #280). `email`
    /// is what a client from before usernames calls the same field.
    #[serde(alias = "email")]
    username: String,
    password: String,
}

/// How long a confirmation link is good for.
pub(crate) const CONFIRM_TTL_SECS: u64 = 24 * 3600;

/// Mints a confirmation link for an account and mails it.
///
/// Returns without sending on a gateway with no mailer. The address then
/// stays unconfirmed, which costs its owner nothing: it keeps nobody out
/// (#269).
pub(crate) async fn mail_confirmation(state: &Shared, account_id: &str) {
    if !state.mail.required() {
        return;
    }
    let issued = auth::IssuedToken::new();
    let now = auth::now_secs();
    // The old link stops working before the new one is written: a resend
    // that left both alive would be two working logins in one mailbox.
    if let Err(e) = store::clear_confirmations(&state.db, account_id).await {
        tracing::error!("{e:#}");
        return;
    }
    // An account without an address has nowhere to be sent a link.
    let Ok(Some(account)) = store::account(&state.db, account_id).await else {
        return;
    };
    let Some(address) = account.email else {
        return;
    };
    let link = Confirmation {
        token_hash: auth::token_digest(&issued.token),
        account_id: account_id.to_string(),
        expires_at: now + CONFIRM_TTL_SECS,
    };
    if let Err(e) = store::put_confirmation(&state.db, link).await {
        tracing::error!("{e:#}");
        return;
    }
    state
        .mail
        .send_confirmation(
            &address,
            &account.display_name,
            &account.lang,
            &issued.token,
        )
        .await;
}

/// The link in the mail.
///
/// Plain text rather than a redirect into the client: the gateway does not
/// know where a client is served from — a native build is not served at all —
/// and a redirect target read off a header is how an open redirect happens.
pub(crate) async fn confirm(
    State(state): State<Shared>,
    Query(query): Query<ConfirmQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let now = auth::now_secs();
    // Read and removed in one step, so a link followed twice works once —
    // including when the second request is a mail client prefetching it.
    let found = store::take_confirmation(&state.db, &auth::token_digest(&query.token))
        .await
        .map_err(|e| db_down(&e))?;
    let Some(found) = found else {
        return Err(err(StatusCode::BAD_REQUEST, "that link is not valid"));
    };
    if found.expires_at <= now {
        return Err(err(StatusCode::BAD_REQUEST, "that link has expired"));
    }
    store::confirm_account(&state.db, &found.account_id, now)
        .await
        .map_err(|e| db_down(&e))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Sends the link again.
///
/// Answers `{"ok":true}` whatever happened, for the same reason registration
/// does: a route that said "no such account" would be an address oracle, and
/// this one needs no password to call.
pub(crate) async fn resend_confirmation(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<ResendBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    if !state
        .limiter
        .allow(&rate_limit_ip(&state.trusted_proxies, addr.ip(), &headers))
    {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    let account_id = store::account_by_email(&state.db, &body.email)
        .await
        .map_err(|e| db_down(&e))?
        .filter(|a| a.confirmed_at.is_none())
        .map(|a| a.id);
    if let Some(account_id) = account_id {
        mail_confirmation(&state, &account_id).await;
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub(crate) async fn register(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<RegisterBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    if !state.registration.takes_sign_ups() {
        return Err(err(StatusCode::FORBIDDEN, "registration is disabled"));
    }
    let ip = rate_limit_ip(&state.trusted_proxies, addr.ip(), &headers);
    if !state.limiter.allow(&ip) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    let Ok(username) = names::username(&body.username) else {
        return Err(err(StatusCode::BAD_REQUEST, "invalid username"));
    };
    if !auth::valid_display_name(&body.display_name) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid display name"));
    }
    if !auth::valid_password(&username.shown, &body.display_name, &body.password) {
        return Err(err(StatusCode::BAD_REQUEST, "invalid password"));
    }
    // Before the expensive hash, so that guessing at keys costs the gateway
    // nothing but a count.
    let key = admission(&state, &ip, body.invite_key.as_deref())?;
    // Argon2 is deliberately expensive and runs off the async worker.
    let password = body.password.clone();
    let password_hash = tokio::task::spawn_blocking(move || auth::hash_password(&password))
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "hashing failed"))?;
    let account = store::NewAccount {
        username: username.shown,
        username_key: username.key,
        display_name: body.display_name,
        password_hash,
        created_at: auth::now_secs(),
        lang: body.lang,
    };
    // The refusal is the unique index's, not a check's. Reading "is this
    // name free" and then writing is two statements another registration
    // can slip between, and both of them would have read "free".
    //
    // And it is said openly (#269). An address could be registered without
    // saying whether it existed, because its owner would get the mail; a
    // username has to be chosen, so a taken one has to be named, and a name
    // that can be chosen can be found. The per-IP limiter above is what
    // bounds that: ten tries in five minutes. What it finds is a login name
    // and no more — the username is shown to nobody but its owner, and the
    // password is still the other half.
    match store::create_account(&state.db, account, key.as_deref())
        .await
        .map_err(|e| db_down(&e))?
    {
        store::Admitted::Made => {
            admitted(&state, &ip, key.as_deref());
            Ok(Json(serde_json::json!({ "ok": true })))
        }
        store::Admitted::Taken => Err(err(StatusCode::CONFLICT, "that username is taken")),
        store::Admitted::KeyRefused => Err(err(StatusCode::FORBIDDEN, invite::KEY_INVALID)),
    }
}

/// The closed-beta key a new account or guest brings (#317), as the hash
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

/// A key admitted somebody: the tries from that address start over, as a
/// right password's do.
pub(crate) fn admitted(state: &AppState, ip: &str, key: Option<&[u8]>) {
    if key.is_some() {
        state.sign_in_limiter.forget(&invite_budget(ip));
    }
}

/// What the sign-in limiter counts an address's key tries under.
pub(crate) fn invite_budget(ip: &str) -> String {
    format!("invite:{ip}")
}

pub(crate) async fn login(
    State(state): State<Shared>,
    Json(creds): Json<Credentials>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    // The account first, so that the tries are counted against it.
    //
    // By address only while there are players who have not yet learnt the
    // name they were given (#269): until 31.12.2026, when #280 removes it,
    // and the answer below tells them their username. A username cannot hold an `@`, so the two
    // never mean the same input. Anything that is not a username at all is
    // simply nobody.
    let account = if creds.username.contains('@') {
        store::account_by_email(&state.db, &creds.username).await
    } else {
        match names::username(&creds.username) {
            Ok(name) => store::account_by_username_key(&state.db, &name.key).await,
            Err(_) => Ok(None),
        }
    }
    .map_err(|e| db_down(&e))?;
    // Eight tries at **one account**, however it was named: by its username
    // and by its address, in any case, the tries are one count. A name that
    // is nobody's is counted under what was typed, so guessing at it is
    // bounded too, and it is answered exactly as a wrong password is.
    let budget = match &account {
        Some(account) => format!("account:{}", account.id),
        None => format!("typed:{}", creds.username.to_lowercase()),
    };
    if !state.sign_in_limiter.allow(&budget) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
    }
    // The expensive verify runs off the async worker, and against a dummy
    // hash when there is no account, so the two take the same time.
    // A guest has no hash, and no username or address to be found by
    // either; were one found, the dummy would refuse it like a stranger.
    let stored_hash = account.as_ref().and_then(|a| a.password_hash.clone());
    let password = creds.password.clone();
    let ok = tokio::task::spawn_blocking(move || {
        auth::verify_password(stored_hash.as_deref(), &password)
    })
    .await
    .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "verify failed"))?;
    let Some(account) = account else {
        return Err(err(StatusCode::UNAUTHORIZED, "invalid credentials"));
    };
    if !ok {
        return Err(err(StatusCode::UNAUTHORIZED, "invalid credentials"));
    }
    // Getting it right is what the window was counting towards. Leaving the
    // typos on the clock would refuse the next sign-in from a player who has
    // just proved who they are.
    state.sign_in_limiter.forget(&budget);
    // An unconfirmed address no longer keeps anyone out (#269): the account
    // signs in with its name, and the address is only a way to reach them.
    let account_id = account.id;
    let issued = auth::IssuedToken::new();
    store::put_token(
        &state.db,
        StoredToken {
            token_hash: auth::token_digest(&issued.token),
            account_id,
            expires_at: issued.expires_at,
        },
    )
    .await
    .map_err(|e| db_down(&e))?;
    Ok(Json(serde_json::json!({
        "token": issued.token,
        "expires_at": issued.expires_at,
        // Its own name, to the one player who may see it: a player who signed
        // in with an address is told the username they were given (#269).
        "username": account.username,
    })))
}

/// What `POST /auth/guest` takes (#269). Every field is optional, and an
/// empty body is a guest called `Guest`.
#[derive(Deserialize, Default)]
pub(crate) struct GuestBody {
    /// The name other players see, under the display-name rule.
    #[serde(default)]
    display_name: Option<String>,
    /// The language it asks in.
    #[serde(default)]
    lang: String,
    /// The closed-beta key a new guest needs on a gateway with
    /// `BAYLEE_REGISTRATION=invite` (#317).
    #[serde(default)]
    invite_key: Option<String>,
}

/// The name a guest is seen by when it chose none.
pub(crate) const GUEST_NAME: &str = "Guest";

/// `POST /auth/guest`: an account with no username, no address and no
/// password, and its session, in one answer (#269).
///
/// The session is the guest: nothing signs in as one, so it lives as long
/// as its session does ([`auth::Lifetime::GUEST`], 29 to 30 days from the
/// last call made with its token) and goes with it, decks included
/// ([`store::purge_guests`]). Bounded twice: per address by the limiter registration uses, and in all
/// by [`AppState::guest_cap`]. The count is read before the write, so a
/// burst of requests at the edge can pass it by as many as are in flight;
/// the cap bounds a flood, not a queue.
pub(crate) async fn guest(
    State(state): State<Shared>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<GuestBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    if !state.guests_enabled {
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
    Ok(Json(serde_json::json!({
        "token": issued.token,
        "expires_at": issued.expires_at,
        "guest": true,
        "handle": handle::handle(&account.display_name, account.tag),
    })))
}

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
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/guest", post(guest))
        .route("/auth/confirm", get(confirm))
        .route("/auth/confirm/resend", post(resend_confirmation))
        .route("/auth/logout", post(logout))
        .route("/account", delete(account::delete_account))
        .route("/me", get(me))
        .route("/players/{handle}", get(player))
        .route("/settings", get(get_settings).put(put_settings))
}

/// The authenticated account's profile.
pub(crate) async fn me(
    State(state): State<Shared>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let account_id = authed(&state, &headers).await?;
    let account = store::account(&state.db, &account_id)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "account gone"))?;
    Ok(Json(serde_json::json!({
        "id": account.id,
        "email": account.email,
        // Only ever to its owner: nothing that shows one player to another
        // carries it (#269).
        "username": account.username,
        // A guest's client says what a guest is (#269): gone about thirty
        // days after its last visit.
        "guest": account.guest,
        "display_name": account.display_name,
        // The two halves separately, because this is the one caller that
        // wants them apart: a settings screen shows the name in a field a
        // player can edit and the tag beside it as something they cannot.
        "tag": handle::tag_text(account.tag),
        "handle": handle::handle(&account.display_name, account.tag),
    })))
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
pub(crate) async fn player(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(typed): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorBody>)> {
    let _ = authed(&state, &headers).await?;
    let tag = handle::parse_tag(&typed)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "a handle carries a #tag"))?;
    let found = store::account_by_tag(&state.db, tag)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such player"))?;
    Ok(Json(serde_json::json!({
        "id": found.id,
        "display_name": found.display_name,
        "tag": handle::tag_text(found.tag),
        "handle": handle::handle(&found.display_name, found.tag),
    })))
}
