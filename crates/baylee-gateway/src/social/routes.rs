//! The HTTP routes under `/social/…` (§8.3). Every one needs a session in
//! `Authorization`, and every one is `403 {"error":"this gateway has no
//! chat"}` when `BAYLEE_SOCIAL=off`. A guest has no friends list, groups or
//! table invites (`403`); it may register its one device, fetch keys, block
//! and set its settings.

use super::invites::Invite;
use super::{members, person, presence, refresh_friends, socket::group_info, tag_number, uid};
use crate::{ErrorBody, Shared, authed_session, db_down, err, store};
use axum::Router;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use axum::routing::{delete, get, post, put};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use baylee_db::social::{self as db, Who};
use baylee_protocol::social::{
    self as wire, AccountKeys, Conv, Created, DeviceKey, DeviceList, DeviceRegistration, DeviceRow,
    Down, FriendEvent, FriendRequest, GroupEvent, GroupName, InviteDecline, InviteKind,
    InviteRequest, KeysAnswer, Notice, PlayerLookup, RequestAnswer, RequestState, SocialSettings,
    TableBrief, TagBody,
};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

type Refusal = (StatusCode, Json<ErrorBody>);
type Answer<T> = Result<T, Refusal>;

/// The routes.
pub(crate) fn routes() -> Router<Shared> {
    Router::new()
        .route("/social/devices", post(register_device).get(list_devices))
        .route("/social/devices/{id}", delete(remove_device))
        .route("/social/keys", get(keys))
        .route("/social/players/{handle}", get(player))
        .route("/social/friends/requests", post(request_friend))
        .route("/social/friends/requests/{tag}/accept", post(accept))
        .route("/social/friends/requests/{tag}/decline", post(decline))
        .route("/social/friends/requests/{tag}", delete(cancel))
        .route("/social/friends/{tag}", delete(unfriend))
        .route("/social/blocks", post(block))
        .route("/social/blocks/{tag}", delete(unblock))
        .route("/social/settings", put(put_settings))
        .route("/social/groups", post(create_group))
        .route(
            "/social/groups/{id}",
            axum::routing::patch(rename_group).delete(delete_group),
        )
        .route("/social/groups/{id}/invites", post(invite_to_group))
        .route("/social/groups/{id}/join", post(join_group))
        .route(
            "/social/groups/{id}/invites/{tag}",
            delete(drop_group_invite),
        )
        .route("/social/groups/{id}/members/{tag}", delete(remove_member))
        .route("/social/groups/{id}/owner", post(hand_over))
        .route("/social/invites", post(invite_to_table))
        .route("/social/invites/{id}", delete(close_invite))
        .route("/social/reports", post(super::report::post_report))
}

/// The caller of a social route.
pub(crate) struct Caller {
    pub(crate) account: String,
    pub(crate) id: Uuid,
    pub(crate) guest: bool,
}

/// The session's account, when the gateway has chat.
pub(crate) async fn caller(state: &Shared, headers: &HeaderMap) -> Answer<Caller> {
    if !state.social.on() {
        return Err(err(StatusCode::FORBIDDEN, "this gateway has no chat"));
    }
    let session = authed_session(state, headers).await?;
    let id = uid(&session.account_id)
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))?;
    Ok(Caller {
        account: session.account_id,
        id,
        guest: session.guest,
    })
}

/// A guest is refused the friends list, groups and invites.
fn registered(caller: &Caller) -> Answer<()> {
    if caller.guest {
        Err(err(StatusCode::FORBIDDEN, "guests have no friends list"))
    } else {
        Ok(())
    }
}

fn down(e: impl Into<anyhow::Error>) -> Refusal {
    db_down(&e.into())
}

/// The account a tag names, or 404.
async fn by_tag(state: &Shared, tag: &str) -> Answer<Who> {
    let number = tag_number(tag).ok_or_else(|| err(StatusCode::NOT_FOUND, "no such player"))?;
    db::who_by_tag(&state.db, number)
        .await
        .map_err(down)?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such player"))
}

/// `frame` to every account in `accounts`.
fn push_all<'a>(state: &Shared, accounts: impl IntoIterator<Item = &'a String>, frame: &Down) {
    for account in accounts {
        state.social.push(account, frame);
    }
}

/// `KeysChanged { tag }` to `account` itself and to everyone who may write
/// to it.
async fn keys_changed(state: &Shared, account: Uuid, tag: &str) {
    let frame = Down::KeysChanged {
        tag: tag.to_owned(),
    };
    match members::may_write_to(state, account).await {
        Ok(accounts) => push_all(state, &accounts, &frame),
        Err(e) => tracing::warn!("social: {e:#}"),
    }
}

// ---------------------------------------------------------------------------
// Devices and keys.
// ---------------------------------------------------------------------------

fn b64(field: &str, len: usize) -> Answer<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(field)
        .ok()
        .filter(|bytes| bytes.len() == len)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "a key has the wrong shape"))
}

/// A device's label: the OS family, 1 to 24 characters, nothing hidden.
fn label_ok(label: &str) -> bool {
    let count = label.chars().count();
    (1..=wire::MAX_DEVICE_LABEL).contains(&count)
        && !label.trim().is_empty()
        && !label
            .chars()
            .any(|c| c.is_control() || crate::routes::is_bidi_control(c) || invisible(c))
}

fn invisible(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}' | '\u{061C}' | '\u{180E}' | '\u{200B}'..='\u{200F}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}'
    )
}

/// `POST /social/devices`.
///
/// @route POST /social/devices
/// @scope public
/// @operation-id `social_register_device`
/// @summary Register a chat device
/// @description Registers one of the account's chat devices by its two
/// public keys and the identity key's statement over them, and answers
/// its id. The same device again (a client registers on each start) is
/// answered the same and costs nothing; a guest has one device, which a new
/// one replaces. Everyone who may write to the account is told its keys
/// changed.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @return 201: `$Created` "The device's id"
/// @return 400: `$ErrorBody` "A key of the wrong shape, a label that is not an OS family, or a device statement that does not hold for the account's tag"
/// @return 403: `$ErrorBody` "This gateway has no chat, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 409: `$ErrorBody` "The account has five devices already, or another device has this key"
/// @return 429: `$ErrorBody` "Five new devices today already"
async fn register_device(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<DeviceRegistration>,
) -> Answer<(StatusCode, Json<Created>)> {
    let caller = caller(&state, &headers).await?;
    let ik = b64(&body.ik, 32)?;
    let dk = b64(&body.dk, 32)?;
    let dsig = b64(&body.dsig, 64)?;
    if !label_ok(&body.label) {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "a device's label is its OS family",
        ));
    }
    let account = store::account(&state.db, &caller.account)
        .await
        .map_err(|e| db_down(&e))?
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))?;
    let tag = crate::handle::tag_text(account.tag);
    let (Ok(ik_arr), Ok(dk_arr), Ok(dsig_arr)) = (
        <[u8; 32]>::try_from(ik.as_slice()),
        <[u8; 32]>::try_from(dk.as_slice()),
        <[u8; 64]>::try_from(dsig.as_slice()),
    ) else {
        return Err(err(StatusCode::BAD_REQUEST, "a key has the wrong shape"));
    };
    if baylee_seal::verify_device(&tag, &ik_arr, &dk_arr, &dsig_arr).is_err() {
        return Err(err(
            StatusCode::BAD_REQUEST,
            "the device's statement does not hold",
        ));
    }
    let id = baylee_seal::device_id(&ik_arr);
    let max = if caller.guest {
        wire::GUEST_DEVICES
    } else {
        wire::MAX_DEVICES
    };
    // The day's allowance counts new devices only: the same device again
    // (a client registering on each start) is free, and a full account is
    // told it is full rather than that it asked too often.
    let have = db::devices_of(&state.db, &[caller.id])
        .await
        .map_err(down)?;
    let again = have.iter().any(|d| d.id == id);
    if !again && !caller.guest && have.len() >= max {
        return Err(err(
            StatusCode::CONFLICT,
            "this account has as many devices as it may",
        ));
    }
    if !again && !state.social.registrations.allow(&caller.account) {
        return Err(err(
            StatusCode::TOO_MANY_REQUESTS,
            "too many new devices today",
        ));
    }
    let new = db::NewDevice {
        id: id.clone(),
        ik,
        dk,
        dsig,
        label: body.label,
    };
    match db::register_device(&state.db, caller.id, &new, max, caller.guest)
        .await
        .map_err(down)?
    {
        db::Registered::Added { replaced } => {
            for old in &replaced {
                state.social.close_device(old, 4002, "device replaced");
            }
            keys_changed(&state, caller.id, &tag).await;
        }
        db::Registered::Again => {}
        db::Registered::Full => {
            return Err(err(
                StatusCode::CONFLICT,
                "this account has as many devices as it may",
            ));
        }
        db::Registered::Taken => {
            return Err(err(StatusCode::CONFLICT, "that key is another device's"));
        }
    }
    Ok((StatusCode::CREATED, Json(Created { id })))
}

/// `GET /social/devices`.
///
/// @route GET /social/devices
/// @scope public
/// @operation-id `social_list_devices`
/// @summary List the account's chat devices
/// @description The account's chat devices, oldest first: their ids,
/// OS families, and when each was registered and last seen.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @return 200: `$DeviceList` "The devices"
/// @return 403: `$ErrorBody` "This gateway has no chat, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
async fn list_devices(State(state): State<Shared>, headers: HeaderMap) -> Answer<Json<DeviceList>> {
    let caller = caller(&state, &headers).await?;
    let devices = db::devices_of(&state.db, &[caller.id])
        .await
        .map_err(down)?
        .into_iter()
        .map(|d| DeviceRow {
            id: d.id,
            label: d.label,
            added_at: super::millis(d.added_at),
            seen_at: super::millis(d.seen_at),
        })
        .collect();
    Ok(Json(DeviceList { devices }))
}

/// `DELETE /social/devices/{id}`.
///
/// @route DELETE /social/devices/{id}
/// @scope public
/// @operation-id `social_remove_device`
/// @summary Remove a chat device
/// @description Removes one of the account's chat devices; its social
/// socket closes (4003), and everyone who may write to the account is told
/// its keys changed.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The device id"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such device of this account"
async fn remove_device(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    if !db::remove_device(&state.db, caller.id, &id)
        .await
        .map_err(down)?
    {
        return Err(err(StatusCode::NOT_FOUND, "no such device"));
    }
    state.social.close_device(&id, 4003, "device removed");
    if let Ok(Some(account)) = store::account(&state.db, &caller.account).await {
        keys_changed(&state, caller.id, &crate::handle::tag_text(account.tag)).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

/// The tags a query names: `tag=a&tag=b`, or `tag=a,b`.
fn tags_in(query: Option<&str>) -> Vec<String> {
    let mut tags = Vec::new();
    for pair in query.unwrap_or_default().split('&') {
        if let Some(value) = pair.strip_prefix("tag=") {
            tags.extend(
                value
                    .split([',', '+'])
                    .filter(|t| !t.is_empty())
                    .map(str::to_owned),
            );
        }
    }
    tags.retain(|t| {
        // `%2C` for a comma written by a strict encoder.
        !t.contains('%')
    });
    tags
}

/// `GET /social/keys?tag=…`: the devices of the tags this account may
/// write to; any other tag is left out rather than refused.
///
/// @route GET /social/keys
/// @scope public
/// @operation-id `social_keys`
/// @summary Fetch the keys of accounts to write to
/// @description The public keys of every device of the accounts named by
/// tag that the caller may write to: itself, its friends, its groups'
/// members and the players at its tables. Any other tag is left out
/// rather than refused. At most 60 tags, as `tag=a&tag=b` or `tag=a,b`.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @query-param tag: Option<Vec<String>> "The accounts' tags; repeated, or comma-separated"
/// @return 200: `$KeysAnswer` "The keys of the accounts the caller may write to"
/// @return 400: `$ErrorBody` "More than 60 tags"
/// @return 403: `$ErrorBody` "This gateway has no chat, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 429: `$ErrorBody` "Past 20 key fetches at once, then one a second"
async fn keys(
    State(state): State<Shared>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Answer<Json<KeysAnswer>> {
    let caller = caller(&state, &headers).await?;
    if !state
        .social
        .admit_keys(&caller.account, std::time::Instant::now())
    {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many key fetches"));
    }
    let mut tags = tags_in(query.as_deref());
    tags.sort();
    tags.dedup();
    if tags.len() > wire::MAX_KEY_TAGS {
        return Err(err(StatusCode::BAD_REQUEST, "at most 60 tags at once"));
    }
    let numbers: Vec<i32> = tags.iter().filter_map(|t| tag_number(t)).collect();
    let may = members::may_write_to(&state, caller.id)
        .await
        .map_err(|e| db_down(&e))?;
    let whos: Vec<Who> = db::who_by_tags(&state.db, &numbers)
        .await
        .map_err(down)?
        .into_iter()
        .filter(|who| may.contains(&who.id.to_string()))
        .collect();
    let ids: Vec<Uuid> = whos.iter().map(|w| w.id).collect();
    let devices = db::devices_of(&state.db, &ids).await.map_err(down)?;
    let mut by_account: BTreeMap<Uuid, Vec<DeviceKey>> = BTreeMap::new();
    for d in devices {
        by_account.entry(d.account_id).or_default().push(DeviceKey {
            id: d.id,
            ik: URL_SAFE_NO_PAD.encode(&d.ik),
            dk: URL_SAFE_NO_PAD.encode(&d.dk),
            dsig: URL_SAFE_NO_PAD.encode(&d.dsig),
            label: d.label,
            added_at: super::millis(d.added_at),
        });
    }
    let keys = whos
        .iter()
        .map(|who| {
            let p = person(who);
            AccountKeys {
                tag: p.tag,
                handle: p.handle,
                devices: by_account.remove(&who.id).unwrap_or_default(),
            }
        })
        .collect();
    Ok(Json(KeysAnswer { keys }))
}

/// `GET /social/players/{handle}`: a sibling of `GET /players/{handle}`
/// that never carries an id.
///
/// @route GET /social/players/{handle}
/// @scope public
/// @operation-id `social_player`
/// @summary Look a player up by handle
/// @description A player by handle (`Alice#af03`, or `#af03`; either
/// case), with how the two stand: friends, a request either way, or
/// blocked. Never an account id.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param handle: String "A handle, `Name#tag` or `#tag`, its `#` percent-encoded (`%23`)"
/// @return 200: `$PlayerLookup` "The player"
/// @return 400: `$ErrorBody` "The handle carries no `#tag`"
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such player"
async fn player(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(typed): Path<String>,
) -> Answer<Json<PlayerLookup>> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let tag = crate::handle::parse_tag(&typed)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "a handle carries a #tag"))?;
    let who = db::who_by_tag(&state.db, tag)
        .await
        .map_err(down)?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such player"))?;
    let relation = db::relation(&state.db, caller.id, who.id)
        .await
        .map_err(down)?;
    let p = person(&who);
    Ok(Json(PlayerLookup {
        tag: p.tag,
        handle: p.handle,
        friend: relation.friend,
        incoming: relation.incoming,
        outgoing: relation.outgoing,
        blocked: relation.blocked,
    }))
}

// ---------------------------------------------------------------------------
// Friends and blocks.
// ---------------------------------------------------------------------------

/// The caller's own handle, as others see it.
async fn me(state: &Shared, caller: &Caller) -> Answer<Who> {
    db::who_by_ids(&state.db, &[caller.id])
        .await
        .map_err(down)?
        .into_iter()
        .next()
        .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))
}

fn friend_frame(event: FriendEvent, who: &Who, presence: Option<wire::Presence>) -> Down {
    Down::Friend {
        event,
        person: person(who),
        presence,
    }
}

/// Both are friends now: each is told, with the other's presence.
async fn now_friends(state: &Shared, a: &Who, b: &Who) {
    refresh_friends(state, &[a.id, b.id]).await;
    let (sa, sb) = (a.id.to_string(), b.id.to_string());
    state.social.push(
        &sa,
        &friend_frame(
            FriendEvent::Accepted,
            b,
            Some(presence::presence_now(state, &sb)),
        ),
    );
    state.social.push(
        &sb,
        &friend_frame(
            FriendEvent::Accepted,
            a,
            Some(presence::presence_now(state, &sa)),
        ),
    );
}

/// `POST /social/friends/requests`.
///
/// @route POST /social/friends/requests
/// @scope public
/// @operation-id `social_request_friend`
/// @summary Ask a player to be friends
/// @description Asks a player to be friends, by handle. When they had
/// asked already, the two are friends at once. A request to someone who
/// blocks the caller looks sent and never arrives.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @return 201: `$RequestAnswer` "`pending`, or `friends` when they had asked already"
/// @return 400: `$ErrorBody` "The handle carries no `#tag`, or it is the caller's own"
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), the player is a guest or takes no friend requests, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such player"
/// @return 409: `$ErrorBody` "Already friends, or too many friends (300) or open requests (50)"
/// @return 429: `$ErrorBody` "Past 20 friend requests an hour"
async fn request_friend(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<FriendRequest>,
) -> Answer<(StatusCode, Json<RequestAnswer>)> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let tag = crate::handle::parse_tag(&body.handle)
        .ok_or_else(|| err(StatusCode::BAD_REQUEST, "a handle carries a #tag"))?;
    let target = db::who_by_tag(&state.db, tag)
        .await
        .map_err(down)?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such player"))?;
    if target.id == caller.id {
        return Err(err(StatusCode::BAD_REQUEST, "that is you"));
    }
    if target.guest {
        return Err(err(StatusCode::FORBIDDEN, "guests have no friends list"));
    }
    if !state.social.requests.allow(&caller.account) {
        return Err(err(
            StatusCode::TOO_MANY_REQUESTS,
            "too many friend requests",
        ));
    }
    let me = me(&state, &caller).await?;
    let outcome = db::request_friend(
        &state.db,
        caller.id,
        target.id,
        wire::MAX_OUTGOING_REQUESTS,
        wire::MAX_FRIENDS,
    )
    .await
    .map_err(down)?;
    let state_now = match outcome {
        db::Requested::Pending => {
            state.social.push(
                &caller.account,
                &friend_frame(FriendEvent::RequestOut, &target, None),
            );
            // Never to someone who blocks the caller: it looks sent and
            // never arrives.
            let blocks_me = db::relation(&state.db, caller.id, target.id)
                .await
                .map_err(down)?
                .blocks_me;
            if !blocks_me {
                state.social.push(
                    &target.id.to_string(),
                    &friend_frame(FriendEvent::RequestIn, &me, None),
                );
            }
            RequestState::Pending
        }
        db::Requested::Friends => {
            now_friends(&state, &me, &target).await;
            RequestState::Friends
        }
        db::Requested::AlreadyFriends => {
            return Err(err(StatusCode::CONFLICT, "already friends"));
        }
        db::Requested::Closed => {
            return Err(err(
                StatusCode::FORBIDDEN,
                "this player takes no friend requests",
            ));
        }
        db::Requested::TooMany => {
            return Err(err(
                StatusCode::CONFLICT,
                "too many friends or open requests",
            ));
        }
    };
    Ok((
        StatusCode::CREATED,
        Json(RequestAnswer { state: state_now }),
    ))
}

/// `POST /social/friends/requests/{tag}/accept`.
///
/// @route POST /social/friends/requests/{tag}/accept
/// @scope public
/// @operation-id `social_accept_friend`
/// @summary Accept a friend request
/// @description Accepts the friend request this player sent; both are
/// told, each with the other's presence.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param tag: String "The account's tag: lowercase hex, four to eight digits"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such player, or no request from them"
/// @return 409: `$ErrorBody` "Too many friends (300)"
async fn accept(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(tag): Path<String>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let from = by_tag(&state, &tag).await?;
    match db::accept(&state.db, caller.id, from.id, wire::MAX_FRIENDS)
        .await
        .map_err(down)?
    {
        db::Accepted::Friends => {
            let me = me(&state, &caller).await?;
            now_friends(&state, &me, &from).await;
            Ok(StatusCode::NO_CONTENT)
        }
        db::Accepted::NoRequest => Err(err(StatusCode::NOT_FOUND, "no such request")),
        db::Accepted::TooMany => Err(err(StatusCode::CONFLICT, "too many friends")),
    }
}

/// `POST /social/friends/requests/{tag}/decline`.
///
/// @route POST /social/friends/requests/{tag}/decline
/// @scope public
/// @operation-id `social_decline_friend`
/// @summary Decline a friend request
/// @description Declines the friend request this player sent; both are told.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param tag: String "The account's tag: lowercase hex, four to eight digits"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such player, or no request from them"
async fn decline(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(tag): Path<String>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let from = by_tag(&state, &tag).await?;
    let relation = db::relation(&state.db, caller.id, from.id)
        .await
        .map_err(down)?;
    if !relation.incoming {
        return Err(err(StatusCode::NOT_FOUND, "no such request"));
    }
    db::drop_request(&state.db, from.id, caller.id)
        .await
        .map_err(down)?;
    let me = me(&state, &caller).await?;
    state.social.push(
        &caller.account,
        &friend_frame(FriendEvent::RequestGone, &from, None),
    );
    state.social.push(
        &from.id.to_string(),
        &friend_frame(FriendEvent::RequestGone, &me, None),
    );
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /social/friends/requests/{tag}`: cancels the caller's own.
///
/// @route DELETE /social/friends/requests/{tag}
/// @scope public
/// @operation-id `social_cancel_friend_request`
/// @summary Cancel a friend request
/// @description Cancels the caller's own friend request to this player.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param tag: String "The account's tag: lowercase hex, four to eight digits"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such player, or no request to them"
async fn cancel(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(tag): Path<String>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let to = by_tag(&state, &tag).await?;
    let blocks_me = db::relation(&state.db, caller.id, to.id)
        .await
        .map_err(down)?
        .blocks_me;
    if !db::drop_request(&state.db, caller.id, to.id)
        .await
        .map_err(down)?
    {
        return Err(err(StatusCode::NOT_FOUND, "no such request"));
    }
    let me = me(&state, &caller).await?;
    state.social.push(
        &caller.account,
        &friend_frame(FriendEvent::RequestGone, &to, None),
    );
    if !blocks_me {
        state.social.push(
            &to.id.to_string(),
            &friend_frame(FriendEvent::RequestGone, &me, None),
        );
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /social/friends/{tag}`.
///
/// @route DELETE /social/friends/{tag}
/// @scope public
/// @operation-id `social_unfriend`
/// @summary End a friendship
/// @description Ends the friendship with this player; both are told.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param tag: String "The account's tag: lowercase hex, four to eight digits"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such player, or not friends"
async fn unfriend(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(tag): Path<String>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let other = by_tag(&state, &tag).await?;
    if !db::unfriend(&state.db, caller.id, other.id)
        .await
        .map_err(down)?
    {
        return Err(err(StatusCode::NOT_FOUND, "not friends"));
    }
    let me = me(&state, &caller).await?;
    refresh_friends(&state, &[caller.id, other.id]).await;
    state.social.push(
        &caller.account,
        &friend_frame(FriendEvent::Removed, &other, None),
    );
    state.social.push(
        &other.id.to_string(),
        &friend_frame(FriendEvent::Removed, &me, None),
    );
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /social/blocks`: also ends any friendship and requests both ways.
///
/// @route POST /social/blocks
/// @scope public
/// @operation-id `social_block`
/// @summary Block a player
/// @description Blocks a player, by tag, which also ends a friendship and
/// the requests both ways. Their messages are no longer delivered to the
/// caller; they are told that a friendship or request ended, never of the
/// block.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @return 204:
/// @return 400: `$ErrorBody` "That is the caller"
/// @return 403: `$ErrorBody` "This gateway has no chat, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such player"
async fn block(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<TagBody>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    let other = by_tag(&state, &body.tag).await?;
    if other.id == caller.id {
        return Err(err(StatusCode::BAD_REQUEST, "that is you"));
    }
    let ended = db::block(&state.db, caller.id, other.id)
        .await
        .map_err(down)?;
    let me = me(&state, &caller).await?;
    let them = other.id.to_string();
    refresh_friends(&state, &[caller.id, other.id]).await;
    state.social.push(
        &caller.account,
        &friend_frame(FriendEvent::Blocked, &other, None),
    );
    if ended.was_friend {
        state
            .social
            .push(&them, &friend_frame(FriendEvent::Removed, &me, None));
    }
    if ended.had_outgoing || ended.had_incoming {
        state
            .social
            .push(&them, &friend_frame(FriendEvent::RequestGone, &me, None));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /social/blocks/{tag}`.
///
/// @route DELETE /social/blocks/{tag}
/// @scope public
/// @operation-id `social_unblock`
/// @summary Unblock a player
/// @description Lifts the caller's block of this player.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param tag: String "The account's tag: lowercase hex, four to eight digits"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such player, or not blocked"
async fn unblock(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(tag): Path<String>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    let other = by_tag(&state, &tag).await?;
    if !db::unblock(&state.db, caller.id, other.id)
        .await
        .map_err(down)?
    {
        return Err(err(StatusCode::NOT_FOUND, "not blocked"));
    }
    state.social.push(
        &caller.account,
        &friend_frame(FriendEvent::Unblocked, &other, None),
    );
    Ok(StatusCode::NO_CONTENT)
}

/// `PUT /social/settings`.
///
/// @route PUT /social/settings
/// @scope public
/// @operation-id `social_put_settings`
/// @summary Set the account's social settings
/// @description Sets who may whisper the account, whether it takes friend
/// requests, and whether friends see which table it is at.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
async fn put_settings(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<SocialSettings>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    db::put_settings(
        &state.db,
        caller.id,
        db::Settings {
            whispers_from: body.whispers_from.code(),
            requests_open: body.requests_open,
            share_table: body.share_table,
        },
    )
    .await
    .map_err(down)?;
    state
        .social
        .set_share_table(&caller.account, body.share_table);
    presence::publish(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Groups.
// ---------------------------------------------------------------------------

fn group_conv(id: Uuid) -> String {
    Conv::Group { id: id.to_string() }.to_string()
}

/// The group `id`, or 404.
async fn load_group(state: &Shared, id: &str) -> Answer<db::Group> {
    let id = Uuid::parse_str(id).map_err(|_| err(StatusCode::NOT_FOUND, "no such group"))?;
    db::group(&state.db, id)
        .await
        .map_err(down)?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such group"))
}

/// The members and invitees of `group`, as account ids.
fn everyone(group: &db::Group) -> BTreeSet<String> {
    group
        .members
        .iter()
        .map(|(who, _)| who.id.to_string())
        .chain(group.invited.iter().map(|who| who.id.to_string()))
        .collect()
}

/// After a change: members see it `Updated` and invitees as an invite.
async fn group_moved(state: &Shared, id: Uuid, notice: Option<Notice>) {
    let Ok(Some(group)) = db::group(&state.db, id).await else {
        return;
    };
    let info = group_info(&group);
    let members: Vec<String> = group
        .members
        .iter()
        .map(|(w, _)| w.id.to_string())
        .collect();
    push_all(
        state,
        &members,
        &Down::Group {
            event: GroupEvent::Updated,
            group: info,
            inviter: None,
        },
    );
    if let Some(notice) = notice {
        push_all(
            state,
            &members,
            &Down::Notice {
                conv: group_conv(id),
                notice,
            },
        );
    }
}

/// `POST /social/groups`.
///
/// @route POST /social/groups
/// @scope public
/// @operation-id `social_create_group`
/// @summary Make a group
/// @description Makes a group with the caller as its owner and only
/// member, and answers its id; its conversation is `grp:<id>`.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @return 201: `$Created` "The group's id"
/// @return 400: `$ErrorBody` "The name is not 1 to 48 characters, or hides characters"
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 409: `$ErrorBody` "The caller is in 20 groups already"
/// @return 429: `$ErrorBody` "Past 10 new groups a day"
async fn create_group(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<GroupName>,
) -> Answer<(StatusCode, Json<Created>)> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let name = baylee_protocol::names::group_name(&body.name).map_err(|_| {
        err(
            StatusCode::BAD_REQUEST,
            "a group's name is 1 to 48 characters",
        )
    })?;
    if !state.social.new_groups.allow(&caller.account) {
        return Err(err(
            StatusCode::TOO_MANY_REQUESTS,
            "too many new groups today",
        ));
    }
    let id = db::create_group(&state.db, caller.id, &name, wire::MAX_GROUPS)
        .await
        .map_err(down)?
        .ok_or_else(|| err(StatusCode::CONFLICT, "in as many groups as one may be"))?;
    if let Ok(Some(group)) = db::group(&state.db, id).await {
        state.social.push(
            &caller.account,
            &Down::Group {
                event: GroupEvent::Joined,
                group: group_info(&group),
                inviter: None,
            },
        );
    }
    Ok((StatusCode::CREATED, Json(Created { id: id.to_string() })))
}

/// `PATCH /social/groups/{id}`: the owner renames it.
///
/// @route PATCH /social/groups/{id}
/// @scope public
/// @operation-id `social_rename_group`
/// @summary Rename a group
/// @description The group's owner renames it; its members are told.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The group's id (a UUID)"
/// @return 204:
/// @return 400: `$ErrorBody` "The name is not 1 to 48 characters, or hides characters"
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), the caller is not the group's owner, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such group"
async fn rename_group(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<GroupName>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let group = load_group(&state, &id).await?;
    if group.role(caller.id) != Some(true) {
        return Err(err(StatusCode::FORBIDDEN, "only the group's owner may"));
    }
    let name = baylee_protocol::names::group_name(&body.name).map_err(|_| {
        err(
            StatusCode::BAD_REQUEST,
            "a group's name is 1 to 48 characters",
        )
    })?;
    db::rename_group(&state.db, group.id, &name)
        .await
        .map_err(down)?;
    group_moved(&state, group.id, Some(Notice::Renamed { name })).await;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /social/groups/{id}/invites`: a member invites a friend.
///
/// @route POST /social/groups/{id}/invites
/// @scope public
/// @operation-id `social_invite_to_group`
/// @summary Invite a friend to a group
/// @description A member invites one of its friends, by tag; the invitee
/// and the members are told. Inviting someone invited already is answered
/// the same.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The group's id (a UUID)"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), the caller is not a member, or the invitee is not the caller's friend, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such group or player"
/// @return 409: `$ErrorBody` "Already a member, or the group is full (50 members and invites)"
/// @return 429: `$ErrorBody` "Past 30 invites an hour (group and table invites together)"
async fn invite_to_group(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<TagBody>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let group = load_group(&state, &id).await?;
    let invitee = by_tag(&state, &body.tag).await?;
    let friend = db::relation(&state.db, caller.id, invitee.id)
        .await
        .map_err(down)?
        .friend;
    if group.role(caller.id).is_none() || !friend {
        return Err(err(
            StatusCode::FORBIDDEN,
            "members invite their own friends",
        ));
    }
    if !state.social.invite_limiter.allow(&caller.account) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many invites"));
    }
    match db::invite_to_group(
        &state.db,
        group.id,
        invitee.id,
        caller.id,
        wire::MAX_GROUP_MEMBERS,
    )
    .await
    .map_err(down)?
    {
        db::GroupInvited::Done => {
            if let Ok(Some(now)) = db::group(&state.db, group.id).await {
                // Who invited, so the invitee's client lists the invite at
                // once rather than at its next snapshot.
                let inviter = me(&state, &caller).await.ok().map(|who| person(&who));
                state.social.push(
                    &invitee.id.to_string(),
                    &Down::Group {
                        event: GroupEvent::Invited,
                        group: group_info(&now),
                        inviter,
                    },
                );
            }
            group_moved(&state, group.id, None).await;
            Ok(StatusCode::NO_CONTENT)
        }
        db::GroupInvited::Again => Ok(StatusCode::NO_CONTENT),
        db::GroupInvited::Member => Err(err(StatusCode::CONFLICT, "already a member")),
        db::GroupInvited::Full => Err(err(StatusCode::CONFLICT, "the group is full")),
    }
}

/// `POST /social/groups/{id}/join`: accepts the caller's invite.
///
/// @route POST /social/groups/{id}/join
/// @scope public
/// @operation-id `social_join_group`
/// @summary Join a group
/// @description Accepts the caller's invite to the group; its members are
/// told.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The group's id (a UUID)"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such group, or no invite to it"
/// @return 409: `$ErrorBody` "The caller is in 20 groups already"
async fn join_group(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let group = load_group(&state, &id).await?;
    match db::join_group(&state.db, group.id, caller.id, wire::MAX_GROUPS)
        .await
        .map_err(down)?
    {
        db::Joined::Joined => {
            if let Ok(Some(now)) = db::group(&state.db, group.id).await {
                state.social.push(
                    &caller.account,
                    &Down::Group {
                        event: GroupEvent::Joined,
                        group: group_info(&now),
                        inviter: None,
                    },
                );
            }
            let me = me(&state, &caller).await?;
            group_moved(
                &state,
                group.id,
                Some(Notice::MemberJoined {
                    person: person(&me),
                }),
            )
            .await;
            Ok(StatusCode::NO_CONTENT)
        }
        db::Joined::NoInvite => Err(err(StatusCode::NOT_FOUND, "no invite to this group")),
        db::Joined::TooMany => Err(err(StatusCode::CONFLICT, "in as many groups as one may be")),
    }
}

/// `DELETE /social/groups/{id}/invites/{tag}`: the invitee declines, or the
/// inviter (or the owner) withdraws.
///
/// @route DELETE /social/groups/{id}/invites/{tag}
/// @scope public
/// @operation-id `social_drop_group_invite`
/// @summary Decline or withdraw a group invite
/// @description The invitee declines its invite, or the member who
/// invited (or the owner) withdraws it.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The group's id (a UUID)"
/// @path-param tag: String "The invitee's tag: lowercase hex, four to eight digits"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), the caller is neither the invitee, its inviter nor the owner, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such group, player or invite"
async fn drop_group_invite(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, tag)): Path<(String, String)>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let group = load_group(&state, &id).await?;
    let invitee = by_tag(&state, &tag).await?;
    let inviter = db::group_inviter(&state.db, group.id, invitee.id)
        .await
        .map_err(down)?
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such invite"))?;
    let may =
        invitee.id == caller.id || inviter == caller.id || group.role(caller.id) == Some(true);
    if !may {
        return Err(err(StatusCode::FORBIDDEN, "not your invite"));
    }
    db::drop_group_invite(&state.db, group.id, invitee.id)
        .await
        .map_err(down)?;
    state.social.push(
        &invitee.id.to_string(),
        &Down::Group {
            event: GroupEvent::InviteGone,
            group: group_info(&group),
            inviter: None,
        },
    );
    group_moved(&state, group.id, None).await;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /social/groups/{id}/members/{tag}`: leaving, or the owner
/// removing someone.
///
/// @route DELETE /social/groups/{id}/members/{tag}
/// @scope public
/// @operation-id `social_remove_member`
/// @summary Leave a group, or remove a member
/// @description A member leaves (its own tag), or the owner removes
/// another. An owner who leaves hands the group to the member who joined
/// first; a group nobody is left in is deleted.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The group's id (a UUID)"
/// @path-param tag: String "The member's tag: lowercase hex, four to eight digits"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), the caller removes someone else and is not the owner, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such group or player, or not a member"
async fn remove_member(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path((id, tag)): Path<(String, String)>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let group = load_group(&state, &id).await?;
    let leaving = by_tag(&state, &tag).await?;
    let may = leaving.id == caller.id || group.role(caller.id) == Some(true);
    if !may {
        return Err(err(StatusCode::FORBIDDEN, "only the group's owner may"));
    }
    if group.role(leaving.id).is_none() {
        return Err(err(StatusCode::NOT_FOUND, "not a member"));
    }
    let owner_before = group.members.iter().find(|(_, o)| *o).map(|(w, _)| w.id);
    db::remove_member(&state.db, group.id, leaving.id)
        .await
        .map_err(down)?;
    state.social.push(
        &leaving.id.to_string(),
        &Down::Group {
            event: GroupEvent::Left,
            group: group_info(&group),
            inviter: None,
        },
    );
    group_left(&state, &group, owner_before, &leaving).await;
    Ok(StatusCode::NO_CONTENT)
}

/// What the rest of `group` is told after `leaving` left it.
async fn group_left(state: &Shared, group: &db::Group, owner_before: Option<Uuid>, leaving: &Who) {
    let now = db::group(&state.db, group.id).await.ok().flatten();
    let Some(now) = now else {
        // Nobody left: the invitees learn it is gone.
        let invitees: Vec<String> = group.invited.iter().map(|w| w.id.to_string()).collect();
        push_all(
            state,
            &invitees,
            &Down::Group {
                event: GroupEvent::Deleted,
                group: group_info(group),
                inviter: None,
            },
        );
        return;
    };
    group_moved(
        state,
        group.id,
        Some(Notice::MemberLeft {
            person: person(leaving),
        }),
    )
    .await;
    let owner_now = now.members.iter().find(|(_, o)| *o).map(|(w, _)| w.clone());
    if let Some(owner) = owner_now
        && Some(owner.id) != owner_before
    {
        let members: Vec<String> = now.members.iter().map(|(w, _)| w.id.to_string()).collect();
        push_all(
            state,
            &members,
            &Down::Notice {
                conv: group_conv(group.id),
                notice: Notice::OwnerNow {
                    person: person(&owner),
                },
            },
        );
    }
}

/// `POST /social/groups/{id}/owner`: the owner hands the group on.
///
/// @route POST /social/groups/{id}/owner
/// @scope public
/// @operation-id `social_hand_over_group`
/// @summary Hand a group to another member
/// @description The owner makes another member, by tag, the group's
/// owner; the members are told.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The group's id (a UUID)"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), the caller is not the group's owner, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such group or player, or not a member"
async fn hand_over(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<TagBody>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let group = load_group(&state, &id).await?;
    let to = by_tag(&state, &body.tag).await?;
    if group.role(caller.id) != Some(true) {
        return Err(err(StatusCode::FORBIDDEN, "only the group's owner may"));
    }
    if !db::hand_over(&state.db, group.id, caller.id, to.id)
        .await
        .map_err(down)?
    {
        return Err(err(StatusCode::NOT_FOUND, "not a member"));
    }
    group_moved(
        &state,
        group.id,
        Some(Notice::OwnerNow {
            person: person(&to),
        }),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /social/groups/{id}`: the owner deletes it.
///
/// @route DELETE /social/groups/{id}
/// @scope public
/// @operation-id `social_delete_group`
/// @summary Delete a group
/// @description The owner deletes the group; its members and invitees
/// are told.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The group's id (a UUID)"
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), the caller is not the group's owner, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such group"
async fn delete_group(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let group = load_group(&state, &id).await?;
    if group.role(caller.id) != Some(true) {
        return Err(err(StatusCode::FORBIDDEN, "only the group's owner may"));
    }
    db::delete_group(&state.db, group.id).await.map_err(down)?;
    push_all(
        &state,
        &everyone(&group),
        &Down::Group {
            event: GroupEvent::Deleted,
            group: group_info(&group),
            inviter: None,
        },
    );
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Table invites.
// ---------------------------------------------------------------------------

/// `POST /social/invites`: to the caller's waiting room (join) or running
/// table (watch).
///
/// @route POST /social/invites
/// @scope public
/// @operation-id `social_invite_to_table`
/// @summary Invite a friend to the caller's table
/// @description Invites a friend to the lobby game the caller sits in: to
/// sit down in a waiting room with an open person's chair (`join`), or to
/// watch a running table that allows spectators (`watch`). It stands ten
/// minutes and replaces an earlier invite of that friend to that game; a
/// `join` invite stands in for the room's password at
/// `POST /lobby/games/{id}/join`.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @return 201: `$Created` "The invite's id"
/// @return 403: `$ErrorBody` "This gateway has no chat, the caller is a guest (who has no friends list), the invitee is not a friend, the caller does not sit at that table, or it has no chair or watching to offer, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such player"
/// @return 429: `$ErrorBody` "Past 30 invites an hour (group and table invites together)"
async fn invite_to_table(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(body): Json<InviteRequest>,
) -> Answer<(StatusCode, Json<Created>)> {
    let caller = caller(&state, &headers).await?;
    registered(&caller)?;
    let invitee = by_tag(&state, &body.tag).await?;
    let friend = db::relation(&state.db, caller.id, invitee.id)
        .await
        .map_err(down)?
        .friend;
    if !friend {
        return Err(err(StatusCode::FORBIDDEN, "only friends are invited"));
    }
    let (kind, table) = {
        let lobby = state.lobby.lock();
        let game = lobby
            .games
            .get(&body.game)
            .ok_or_else(|| err(StatusCode::FORBIDDEN, "you do not sit at that table"))?;
        if !members::seated(game, None).contains(&caller.account) {
            return Err(err(StatusCode::FORBIDDEN, "you do not sit at that table"));
        }
        let kind = match game.state {
            crate::LobbyState::Waiting
                if game
                    .seats
                    .iter()
                    .any(|s| s.kind == crate::lobby::SeatKind::Human && !s.occupied()) =>
            {
                InviteKind::Join
            }
            crate::LobbyState::Playing if game.allow_spectators => InviteKind::Watch,
            _ => {
                return Err(err(
                    StatusCode::FORBIDDEN,
                    "that table has no chair or watching to offer",
                ));
            }
        };
        let state_word = match game.state {
            crate::LobbyState::Waiting => "waiting",
            crate::LobbyState::Playing => "playing",
            crate::LobbyState::Over => "over",
        };
        (
            kind,
            TableBrief {
                name: game.name.clone(),
                seats: u8::try_from(game.seats.len()).unwrap_or(u8::MAX),
                taken: u8::try_from(game.seats.iter().filter(|s| s.occupied()).count())
                    .unwrap_or(u8::MAX),
                state: state_word.to_owned(),
                locked: game.password_hash.is_some(),
            },
        )
    };
    if !state.social.invite_limiter.allow(&caller.account) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many invites"));
    }
    let me = me(&state, &caller).await?;
    let now = std::time::Instant::now();
    let invite = Invite {
        id: Uuid::now_v7().to_string(),
        from: caller.account.clone(),
        from_person: person(&me),
        to: invitee.id.to_string(),
        game: body.game,
        kind,
        table,
        issued: now,
    };
    let id = invite.id.clone();
    let frame = Down::Invite(invite.wire(now));
    let to = invite.to.clone();
    let tell = state.social.invites.lock().add(invite);
    for (account, frame) in tell {
        state.social.push(&account, &frame);
    }
    state.social.push(&to, &frame);
    Ok((StatusCode::CREATED, Json(Created { id })))
}

/// `DELETE /social/invites/{id}`: the invitee declines (`{"answer":
/// "declined"}`) or lets it go, or the inviter withdraws it.
///
/// @route DELETE /social/invites/{id}
/// @scope public
/// @operation-id `social_close_invite`
/// @summary Close a table invite
/// @description The invitee declines a table invite (`{"answer":"declined"}`)
/// or lets it go (no body), or the inviter withdraws it; both are told.
/// @tag Social
/// @security bearer
/// @insert `SessionRefusals`
/// @path-param id: String "The invite's id"
/// @body optional `$InviteDecline`
/// @return 204:
/// @return 403: `$ErrorBody` "This gateway has no chat, or the account is banned (`ban`) or must accept the terms in force first (`terms_required`)"
/// @return 404: `$ErrorBody` "No such invite, or the caller is neither its invitee nor its inviter"
async fn close_invite(
    State(state): State<Shared>,
    headers: HeaderMap,
    Path(id): Path<String>,
    body: axum::body::Bytes,
) -> Answer<StatusCode> {
    let caller = caller(&state, &headers).await?;
    let declined = serde_json::from_slice::<InviteDecline>(&body)
        .is_ok_and(|d| d.answer == wire::InviteAnswer::Declined);
    let tell = state
        .social
        .invites
        .lock()
        .close(&id, &caller.account, declined)
        .ok_or_else(|| err(StatusCode::NOT_FOUND, "no such invite"))?;
    for (account, frame) in tell {
        state.social.push(&account, &frame);
    }
    Ok(StatusCode::NO_CONTENT)
}
