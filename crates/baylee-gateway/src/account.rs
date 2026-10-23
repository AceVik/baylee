//! `DELETE /account`: a player deletes their own account (#292).
//!
//! Everything the account owns goes in one statement
//! ([`baylee_db::accounts`]): its sessions, confirmation links, decks with
//! their history, preferences and its claims on uploaded pictures. A picture
//! nobody claims any more is removed, file and all ([`forget_pictures`]).
//! What the gateway holds in memory goes after it: the account's chairs at
//! every table and its open lobby and seat sockets ([`forget_accounts`]).
//!
//! A guest leaves the same way when its last session lapses or it signs out
//! ([`crate::store::purge_guests`]), so both paths go through [`depart`].
//!
//! And as the gateway starts, [`sweep_pictures`] removes the file of every
//! picture nothing claims (#301): the ones uploaded before owners were
//! recorded that no deck showed, and the ones a deletion let go of when the
//! gateway stopped before their files were gone.

use crate::{ErrorBody, Shared, auth, db_down, err, store};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use serde::Deserialize;

/// What `DELETE /account` takes.
/// @openapi
#[derive(Deserialize, Default)]
pub struct Farewell {
    /// @description The account's password, asked again of a registered
    /// account; a guest, or a session of Baylee Central, sends `{}`.
    /// @internal
    /// The account's password, asked again: a session left signed in on
    /// somebody else's machine must not be enough to delete the account. A
    /// guest has none and sends none.
    #[serde(default)]
    password: Option<String>,
}

/// `DELETE /account`: deletes the caller's account and everything it owns,
/// answering 204.
///
/// A registered account names its password again, counted against the same
/// eight tries a sign-in is ([`crate::AppState::sign_in_limiter`]), and a
/// wrong one is 403, not 401: a 401 tells a client its session is spent, and
/// this one is not.
///
/// @route DELETE /account
/// @scope public
/// @summary Delete one's own account
/// @description Deletes the caller's account here and everything it owns:
/// its sessions, decks and their history, preferences and the pictures no
/// other account claims; its chairs and sockets close. A registered account
/// names its password again, counted against the sign-in's eight tries; a
/// guest, or a session of Baylee Central, sends `{}`. Open to a session that
/// has not accepted the terms in force.
/// @tag Accounts
/// @security bearer
/// @insert `SessionRefusalsAnyTerms`
/// @return 204: "The account is deleted"
/// @return 403: `$ErrorBody` "The password is wrong (`wrong password`), or the account is banned (`ban`)"
/// @return 429: `$ErrorBody` "Eight tries in five minutes are spent"
/// @return 500: `$ErrorBody` "The password could not be checked"
pub async fn delete_account(
    State(state): State<Shared>,
    headers: HeaderMap,
    Json(farewell): Json<Farewell>,
) -> Result<StatusCode, (StatusCode, Json<ErrorBody>)> {
    // Open to a session that has not accepted the terms in force: deleting
    // one's data here is one of the three answers the terms sheet offers.
    let session = crate::authed_session_any_terms(&state, &headers).await?;
    // A Central-bound session was just checked live at Baylee Central, and
    // its account has no password here to name again.
    if !session.guest && !session.central {
        let account = store::account(&state.db, &session.account_id)
            .await
            .map_err(|e| db_down(&e))?
            .ok_or_else(|| err(StatusCode::UNAUTHORIZED, "invalid or expired token"))?;
        let budget = format!("account:{}", account.id);
        if !state.sign_in_limiter.allow(&budget) {
            return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many attempts"));
        }
        let stored_hash = account.password_hash.clone();
        let password = farewell.password.unwrap_or_default();
        let ok = tokio::task::spawn_blocking(move || {
            auth::verify_password(stored_hash.as_deref(), &password)
        })
        .await
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "verify failed"))?;
        if !ok {
            return Err(err(StatusCode::FORBIDDEN, "wrong password"));
        }
        // The count names the account, and the account is about to go.
        state.sign_in_limiter.forget(&budget);
    }
    depart(
        &state,
        store::delete_account(&state.db, &session.account_id),
    )
    .await
    .map_err(|e| db_down(&e))?;
    Ok(StatusCode::NO_CONTENT)
}

/// Runs `deletion`, one account's or the guests', and then takes what went
/// out of everything else the gateway holds: the pictures nobody claims
/// any more ([`forget_pictures`]) and the accounts' chairs and sockets
/// ([`forget_accounts`]). Answers what went.
///
/// The image store's lock is held from before the deletion until the files
/// are gone, which is what keeps an upload from losing its file to it
/// ([`crate::cosmetics::Store::hold`]).
///
/// # Errors
///
/// If the deletion fails, in which case nothing went.
pub async fn depart(
    state: &Shared,
    deletion: impl std::future::Future<Output = anyhow::Result<store::Gone>>,
) -> anyhow::Result<store::Gone> {
    let gone = {
        let held = state.deck_images.hold().await;
        let gone = deletion.await?;
        forget_pictures(state, &gone.pictures, &held).await;
        gone
    };
    // A group its owner left passes to the member who joined first, and a
    // group nobody is left in goes (`docs/protocol.md` §"Friends and chat").
    if !gone.accounts.is_empty()
        && let Err(e) = baylee_db::social::reown(&state.db).await
    {
        tracing::warn!("social: groups not handed on: {e}");
    }
    forget_accounts(state, &gone.accounts);
    Ok(gone)
}

/// Removes each of `pictures` that no account claims any more: the file,
/// and any deck's mention of it.
///
/// `_held` is the image store's lock ([`crate::cosmetics::Store::hold`]),
/// held from the deletion that let go of these pictures until each file is
/// gone, so an upload of the same bytes waits and then writes its file
/// anew. A picture that cannot be removed is logged by id and left; the
/// id is a hash of the picture and says nothing about whose it was.
pub async fn forget_pictures(
    state: &Shared,
    pictures: &[String],
    _held: &tokio::sync::MutexGuard<'_, ()>,
) {
    for id in pictures {
        match store::let_go(&state.db, id).await {
            Ok(true) => {
                if let Err(e) = state.deck_images.remove(id) {
                    tracing::error!(id, "an unclaimed picture stayed: {e}");
                }
            }
            Ok(false) => {}
            Err(e) => tracing::error!("{e:#}"),
        }
    }
}

/// Removes the file of every stored picture nothing claims (#301): no
/// owner, and no deck that shows it ([`store::claimed_pictures`]). Run once
/// as the gateway starts; a second run finds nothing to do.
///
/// Under the image store's lock from reading the claims until the files are
/// gone, like an upload and a deletion, so a picture whose owner is being
/// recorded is never swept. What went is logged as a count, not by id.
pub async fn sweep_pictures(state: &Shared) {
    if !state.deck_images.enabled() {
        return;
    }
    let held = state.deck_images.hold().await;
    let claimed = match store::claimed_pictures(&state.db).await {
        Ok(claimed) => claimed,
        Err(e) => {
            tracing::error!("the pictures nothing claims were not swept: {e:#}");
            return;
        }
    };
    let images = std::sync::Arc::clone(&state.deck_images);
    match tokio::task::spawn_blocking(move || images.sweep(&claimed)).await {
        Ok(Ok(swept)) => {
            tracing::info!(removed = swept.removed, "pictures nothing claims swept");
            if swept.failed > 0 {
                tracing::error!(failed = swept.failed, "pictures nothing claims stayed");
            }
        }
        Ok(Err(e)) => tracing::error!("the pictures nothing claims were not swept: {e}"),
        Err(e) => tracing::error!("the pictures nothing claims were not swept: {e}"),
    }
    drop(held);
}

/// Takes deleted accounts out of everything the gateway holds in memory:
/// their chairs at every table ([`crate::lobby::Lobby::forget_account`]),
/// and their open sockets. A seat socket closes because its chair has no
/// seat token any more, a lobby socket because its account is named on
/// [`crate::AppState::departed`].
pub fn forget_accounts(state: &Shared, accounts: &[String]) {
    if accounts.is_empty() {
        return;
    }
    let now = auth::now_secs();
    let mut sat = false;
    {
        let mut lobby = state.lobby.lock();
        for account in accounts {
            sat |= lobby.forget_account(account, now);
        }
    }
    // A chair it handed to a bridge and nobody redeemed yet is not meant any
    // more either (`chair.rs`).
    state.chair_tickets.revoke(|grant| {
        matches!(grant, crate::wsticket::Grant::Chair { host, .. } if accounts.contains(host))
    });
    for account in accounts {
        let _ = state.departed.send(account.clone());
    }
    // Their social sockets close, their friends are told and their table
    // invites lapse.
    state.social.forget(accounts);
    if sat {
        crate::seathost::stop_unseated(state);
        state.lobby_moved();
    }
}
