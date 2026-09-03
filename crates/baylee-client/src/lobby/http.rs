//! The lobby's half of the gateway conversation: one [`LobbyRequest`]
//! becomes one HTTP call, and its answer becomes one [`LobbyEvent`].
//!
//! Split from the state machine on purpose — the route mapping is what a
//! typo breaks, and here it can be read against `docs/protocol.md` in one
//! screen.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

// ------------------------------------------------------------------ HTTP

/// Performs a request the state machine asked for, if it asked for one.
pub(super) fn dispatch(state: &LobbyState, mailbox: &Mailbox, request: Option<LobbyRequest>) {
    let Some(request) = request else {
        return;
    };
    let token = state.lobby.token();
    let (request, expect) = build(&state.gateway, token, &state.lang, request);
    fetch(request, expect, token.is_some(), mailbox);
}

/// The HTTP call one lobby request becomes, and what to make of its answer.
///
/// Separate from [`dispatch`] so the mapping onto the gateway's routes can be
/// tested without a socket: a wrong path or a misspelled field would otherwise
/// only show up as a 404 in somebody's hands.
#[allow(clippy::too_many_lines)] // one arm per route, read top to bottom
pub(super) fn build(
    base: &str,
    token: Option<&str>,
    lang: &str,
    request: LobbyRequest,
) -> (ehttp::Request, Expect) {
    // A gateway URL out of a `.env` file very often ends in one.
    let base = base.trim_end_matches('/');
    let (request, expect) = match request {
        LobbyRequest::Register {
            email,
            display_name,
            password,
        } => (
            json_post(
                &format!("{base}/auth/register"),
                &serde_json::json!({
                    "email": email,
                    "display_name": display_name,
                    "password": password,
                }),
            ),
            Expect::Registered,
        ),
        LobbyRequest::LogIn { email, password } => (
            json_post(
                &format!("{base}/auth/login"),
                &serde_json::json!({ "email": email, "password": password }),
            ),
            Expect::LoggedIn,
        ),
        LobbyRequest::ListDecks => (ehttp::Request::get(format!("{base}/decks")), Expect::Decks),
        LobbyRequest::LoadPool => (
            // The pool is public reference data and needs no token; the lang
            // is what decides whether names and rules text come back
            // translated, and it is the same one the duel reads card text in.
            ehttp::Request::get(format!("{base}/pool?lang={lang}")),
            Expect::Pool,
        ),
        LobbyRequest::LoadPrintings { card } => (
            // Public for the same reason the pool is: which sets a card
            // appeared in is reference data, not something about an account.
            ehttp::Request::get(format!("{base}/printings?card={card}")),
            Expect::Printings,
        ),
        LobbyRequest::LoadDeck { deck_id } => (
            ehttp::Request::get(format!("{base}/decks/{deck_id}")),
            Expect::DeckLoaded,
        ),
        LobbyRequest::SaveDeck {
            deck_id,
            name,
            cards,
            sideboard,
            commander,
        } => {
            let body = serde_json::json!({
                "name": name,
                "cards": cards,
                "sideboard": sideboard,
                "commander": commander,
            });
            match deck_id {
                // Editing an existing deck overwrites it; without an id this
                // is a new one. Getting that backwards would either lose the
                // original or leave a duplicate behind on every save.
                Some(id) => (
                    json_body("PUT", &format!("{base}/decks/{id}"), &body),
                    Expect::DeckSaved,
                ),
                None => (
                    json_post(&format!("{base}/decks"), &body),
                    Expect::DeckSaved,
                ),
            }
        }
        LobbyRequest::DeleteDeck { deck_id } => (
            ehttp::Request {
                method: "DELETE".to_string(),
                ..ehttp::Request::get(format!("{base}/decks/{deck_id}"))
            },
            Expect::DeckDeleted,
        ),
        LobbyRequest::ListGames => (
            ehttp::Request::get(format!("{base}/lobby/games")),
            Expect::Games,
        ),
        LobbyRequest::CreateGame {
            deck_id,
            mode,
            chairs,
            name,
            password,
        } => (
            json_post(
                &format!("{base}/lobby/games"),
                &serde_json::json!({
                    "deck_id": deck_id,
                    "mode": mode.wire(),
                    "seats": chairs,
                    "name": name,
                    "password": password,
                }),
            ),
            Expect::Seat,
        ),
        LobbyRequest::JoinGame {
            game_id,
            deck_id,
            seat,
            password,
        } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/join"),
                &serde_json::json!({
                    "deck_id": deck_id,
                    "seat": seat,
                    "password": password,
                }),
            ),
            Expect::Seat,
        ),
        LobbyRequest::SetSeat {
            game_id,
            seat,
            kind,
            ai,
            deck_id,
        } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/seats/{seat}"),
                &serde_json::json!({
                    "kind": kind.map(|k| match k {
                        SeatKind::Human => "human",
                        SeatKind::Ai => "ai",
                    }),
                    "ai": ai,
                    "deck_id": deck_id,
                }),
            ),
            // Arranging a chair answers with the listing, so the room the
            // player is looking at redraws without a second round trip.
            Expect::Games,
        ),
        // All three answer with the listing, so the room the player is
        // looking at redraws without a second round trip.
        LobbyRequest::SetReady { game_id, ready } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/ready"),
                &serde_json::json!({ "ready": ready }),
            ),
            Expect::Games,
        ),
        LobbyRequest::StartGame { game_id } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/start"),
                &serde_json::json!({}),
            ),
            Expect::Games,
        ),
        LobbyRequest::HandOver { game_id, seat } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/host"),
                &serde_json::json!({ "seat": seat }),
            ),
            Expect::Games,
        ),
        LobbyRequest::LeaveGame { game_id } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/leave"),
                &serde_json::json!({}),
            ),
            Expect::Left,
        ),
    };
    (bearer(request, token), expect)
}

/// A JSON `POST`. Built by hand rather than through `ehttp`'s `json` feature,
/// which would pull serde into a crate that already has it.
///
/// The headers are replaced, not added to: `ehttp`'s `insert` appends, and
/// `Request::post` has already set a `text/plain` content type that axum's
/// `Json` extractor refuses.
fn json_post(url: &str, body: &serde_json::Value) -> ehttp::Request {
    json_body("POST", url, body)
}

/// A JSON request with any method. `ehttp` only builds `GET` and `POST`, and
/// updating a deck is a `PUT`.
fn json_body(method: &str, url: &str, body: &serde_json::Value) -> ehttp::Request {
    let mut request = ehttp::Request::post(url, serde_json::to_vec(body).unwrap_or_default());
    request.method = method.to_string();
    request.headers = ehttp::Headers::new(&[
        ("Accept", "application/json"),
        ("Content-Type", "application/json"),
    ]);
    request
}

/// Signs a request with the account token, when there is one.
fn bearer(mut request: ehttp::Request, token: Option<&str>) -> ehttp::Request {
    if let Some(token) = token {
        request
            .headers
            .insert("Authorization", format!("Bearer {token}"));
    }
    request
}

/// Sends a request and posts its outcome to the mailbox.
fn fetch(request: ehttp::Request, expect: Expect, signed: bool, mailbox: &Mailbox) {
    let box_ = Arc::clone(&mailbox.0);
    ehttp::fetch(request, move |result| {
        let reply = match result {
            Ok(response) if response.ok => Reply::Event(decode(expect, &response)),
            // Only a *signed* 401 means the token is spent; on the sign-in
            // form it means the password was wrong.
            Ok(response) if signed && response.status == 401 => Reply::Expired,
            Ok(response) => Reply::Event(LobbyEvent::Failed(gateway_error(&response))),
            Err(err) => Reply::Event(LobbyEvent::Failed(format!(
                "the gateway did not answer: {err}"
            ))),
        };
        if let Ok(mut box_) = box_.lock() {
            box_.push(reply);
        }
    });
}

/// Turns a successful response into the event the lobby is waiting for.
pub(super) fn decode(expect: Expect, response: &ehttp::Response) -> LobbyEvent {
    /// `POST /auth/login`.
    #[derive(serde::Deserialize)]
    struct TokenBody {
        token: String,
    }

    /// `POST /decks`. An edit answers `204` and parses to nothing.
    #[derive(serde::Deserialize)]
    struct SavedDeck {
        deck_id: String,
    }

    /// `GET /pool`.
    #[derive(serde::Deserialize)]
    struct PoolBody {
        cards: Vec<baylee_client_core::PoolCard>,
        #[serde(default)]
        has_text: bool,
    }

    /// `GET /printings`.
    #[derive(serde::Deserialize)]
    struct PrintingsBody {
        card: u32,
        printings: Vec<baylee_client_core::deckbuilder::Printing>,
        #[serde(default)]
        from_catalog: bool,
    }

    /// `GET /decks/{id}`.
    #[derive(serde::Deserialize)]
    struct StoredDeck {
        id: String,
        name: String,
        cards: Vec<String>,
        #[serde(default)]
        sideboard: Vec<String>,
        #[serde(default)]
        commander: Option<String>,
    }

    let body = response.text().unwrap_or_default();
    match expect {
        Expect::Registered => LobbyEvent::Registered,
        // An edit answers `204` with no body and needs no id: the builder
        // already holds the one it is editing.
        Expect::DeckSaved => LobbyEvent::DeckSaved {
            deck_id: serde_json::from_str::<SavedDeck>(body)
                .ok()
                .map(|d| d.deck_id),
        },
        Expect::DeckDeleted => LobbyEvent::DeckDeleted,
        Expect::Pool => serde_json::from_str::<PoolBody>(body).map_or_else(
            |_| unreadable("the card pool"),
            |b| LobbyEvent::Pool {
                cards: b.cards,
                has_text: b.has_text,
            },
        ),
        Expect::Printings => serde_json::from_str::<PrintingsBody>(body).map_or_else(
            |_| unreadable("the printings"),
            |b| LobbyEvent::Printings {
                card: b.card,
                printings: b.printings,
                from_catalog: b.from_catalog,
            },
        ),
        Expect::DeckLoaded => serde_json::from_str::<StoredDeck>(body).map_or_else(
            |_| unreadable("the deck"),
            |d| LobbyEvent::DeckLoaded {
                id: d.id,
                name: d.name,
                cards: d.cards,
                sideboard: d.sideboard,
                commander: d.commander,
            },
        ),
        Expect::LoggedIn => serde_json::from_str::<TokenBody>(body).map_or_else(
            |_| unreadable("the sign-in"),
            |b| LobbyEvent::LoggedIn { token: b.token },
        ),
        Expect::Decks => serde_json::from_str(body)
            .map_or_else(|_| unreadable("the deck list"), LobbyEvent::Decks),
        Expect::Games => serde_json::from_str(body)
            .map_or_else(|_| unreadable("the game list"), LobbyEvent::Games),
        Expect::Seat => {
            serde_json::from_str(body).map_or_else(|_| unreadable("the seat"), LobbyEvent::Seated)
        }
        // Nothing comes back, so the lobby re-reads the list to find out what
        // the table looks like without us.
        Expect::Left => LobbyEvent::Left,
    }
}

/// The message for a body that arrived but made no sense.
fn unreadable(what: &str) -> LobbyEvent {
    LobbyEvent::Failed(format!("could not read {what} the gateway sent"))
}

/// The gateway's own `{"error":…}`, or the bare status if it sent none.
pub(super) fn gateway_error(response: &ehttp::Response) -> String {
    /// Every refusal the gateway sends has this shape.
    #[derive(serde::Deserialize)]
    struct Body {
        error: String,
    }

    response
        .text()
        .and_then(|body| serde_json::from_str::<Body>(body).ok())
        .map_or_else(
            || format!("the gateway answered {}", response.status),
            |b| b.error,
        )
}

/// Asks once, at startup, whether this gateway takes sign-ups.
pub(super) fn ask_about_registration(state: Res<LobbyState>, mailbox: Res<Mailbox>) {
    /// `GET /auth/config`.
    #[derive(serde::Deserialize)]
    struct Body {
        registration_enabled: bool,
    }

    let box_ = Arc::clone(&mailbox.0);
    let url = format!("{}/auth/config", state.gateway);
    ehttp::fetch(ehttp::Request::get(&url), move |result| {
        let enabled = match result {
            Ok(response) if response.ok => response
                .text()
                .and_then(|body| serde_json::from_str::<Body>(body).ok())
                .map(|b| b.registration_enabled),
            // A gateway that is not up yet says nothing about registration.
            // Leaving the offer standing is the recoverable failure.
            _ => None,
        };
        if let Some(enabled) = enabled
            && let Ok(mut box_) = box_.lock()
        {
            box_.push(Reply::Registration(enabled));
        }
    });
}
