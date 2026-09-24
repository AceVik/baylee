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
///
/// Two performers, one protocol. A gateway answers over a socket and leaves
/// its reply in the mailbox for a later frame; offline answers out of the
/// registry and a file, in this call. The immediate answer is *posted into
/// the mailbox* rather than returned, which is what makes those the same
/// thing to everything above here: no caller has to know which performer it
/// got, and an offline reply goes round exactly the loop a networked one
/// does.
pub(super) fn dispatch(state: &mut LobbyState, mailbox: &Mailbox, request: Option<LobbyRequest>) {
    let Some(request) = request else {
        return;
    };
    // The offline performer writes the words a player reads, so it needs
    // the language the lobby is speaking. A gateway does not: its rows are
    // account names, which have no language.
    let lang = state.lobby.lang();
    let pool = matches!(request, LobbyRequest::LoadPool);
    if let Some(offline) = state.offline.as_mut() {
        let event = offline.perform(request, lang);
        if let Ok(mut box_) = mailbox.0.lock() {
            box_.push(if pool {
                Reply::PoolLanguage(lang, event)
            } else {
                Reply::Event(event)
            });
        }
        return;
    }
    let token = state.lobby.token();
    let (request, expect) = build(&state.gateway, token, &state.lang, request);
    fetch(
        request,
        expect,
        state.lobby.lang(),
        token.is_some(),
        state.gateway_epoch,
        mailbox,
    );
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
                    // What the confirmation mail is written in. The gateway
                    // keeps it on the account, so a later resend still lands
                    // in the language the player signed up in.
                    "lang": lang,
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
        LobbyRequest::Library(request) => library_request(base, request),
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
            commanders,
        } => {
            let body = serde_json::json!({
                "name": name,
                "cards": cards,
                "sideboard": sideboard,
                "commanders": commanders,
            });
            match deck_id {
                // Editing an existing deck overwrites it; without an id this
                // is a new one. Getting that backwards would either lose the
                // original or leave a duplicate behind on every save.
                Some(id) => (
                    json_body(ehttp::Method::PUT, &format!("{base}/decks/{id}"), &body),
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
                method: ehttp::Method::DELETE,
                ..ehttp::Request::get(format!("{base}/decks/{deck_id}"))
            },
            Expect::DeckDeleted,
        ),
        LobbyRequest::ListGames(query) => (
            ehttp::Request::get(format!("{base}/lobby/games?{}", params(&query))),
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
        // A ticket for a chair already held, answered exactly as a join is —
        // which is the whole reason nothing downstream had to learn about it.
        LobbyRequest::TakeSeat { game_id } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/seat"),
                &serde_json::json!({}),
            ),
            Expect::Seat,
        ),
        LobbyRequest::SetSeat {
            game_id,
            seat,
            kind,
            ai,
            deck_id,
            team,
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
                    "team": team,
                }),
            ),
            // The answer says what the whole lobby looks like; this client
            // is reading one page of it, so what it takes from the reply is
            // that something moved.
            Expect::Moved,
        ),
        // All three answer the same way, and for the same reason.
        LobbyRequest::SetReady { game_id, ready } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/ready"),
                &serde_json::json!({ "ready": ready }),
            ),
            Expect::Moved,
        ),
        LobbyRequest::StartGame { game_id } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/start"),
                &serde_json::json!({}),
            ),
            Expect::Moved,
        ),
        LobbyRequest::HandOver { game_id, seat } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/host"),
                &serde_json::json!({ "seat": seat }),
            ),
            Expect::Moved,
        ),
        LobbyRequest::LeaveGame { game_id } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/leave"),
                &serde_json::json!({}),
            ),
            Expect::Left,
        ),
        // A ticket, exactly as a join answers — which is why pressing play
        // again needs nothing downstream of it that a join did not already
        // need.
        LobbyRequest::Rematch { game_id } => (
            json_post(
                &format!("{base}/lobby/games/{game_id}/rematch"),
                &serde_json::json!({}),
            ),
            Expect::Seat,
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
    json_body(ehttp::Method::POST, url, body)
}

/// A JSON request with any method. `ehttp` only builds `GET` and `POST`, and
/// updating a deck is a `PUT`.
fn json_body(method: ehttp::Method, url: &str, body: &serde_json::Value) -> ehttp::Request {
    let mut request = ehttp::Request::post(url, serde_json::to_vec(body).unwrap_or_default());
    request.method = method;
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
fn fetch(
    request: ehttp::Request,
    expect: Expect,
    lang: Lang,
    signed: bool,
    epoch: u64,
    mailbox: &Mailbox,
) {
    let box_ = Arc::clone(&mailbox.0);
    let library = matches!(expect, Expect::Library(_));
    let pool = matches!(expect, Expect::Pool);
    ehttp::fetch(request, move |result| {
        let reply = match result {
            Ok(response) if response.ok => Reply::Event(decode(lang, expect, &response)),
            // Only a *signed* 401 means the token is spent; on the sign-in
            // form it means the password was wrong.
            Ok(response) if signed && response.status == 401 => Reply::Expired,
            Ok(response) => Reply::Event(LobbyEvent::Failed(gateway_error(lang, &response))),
            Err(err) => Reply::Event(LobbyEvent::Failed(
                Phrase::GatewayNoAnswer.fill(lang, &[&err]),
            )),
        };
        let reply = match reply {
            Reply::Event(event @ LobbyEvent::Pool { .. }) if pool => {
                Reply::PoolLanguage(lang, event)
            }
            Reply::Event(LobbyEvent::Failed(error)) if library => Reply::Event(
                LobbyEvent::Library(client_core::lobby::library::Reply::Failed(error)),
            ),
            other => other,
        };
        if let Ok(mut box_) = box_.lock() {
            box_.push(Reply::Remote(epoch, Box::new(reply)));
        }
    });
}

/// Turns a successful response into the event the lobby is waiting for.
pub(super) fn decode(lang: Lang, expect: Expect, response: &ehttp::Response) -> LobbyEvent {
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
        commanders: Vec<String>,
    }

    let body = response.text().unwrap_or_default();
    match expect {
        // A gateway written before confirmation existed sends no such
        // field, and `false` is what it meant: it never asked.
        Expect::Registered => LobbyEvent::Registered {
            confirmation_required: serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|v| v.get("confirmation_required")?.as_bool())
                .unwrap_or(false),
        },
        // An edit answers `204` with no body and needs no id: the builder
        // already holds the one it is editing.
        Expect::Library(request) => decode_library(request, body, lang),
        Expect::DeckSaved => LobbyEvent::DeckSaved {
            deck_id: serde_json::from_str::<SavedDeck>(body)
                .ok()
                .map(|d| d.deck_id),
        },
        Expect::DeckDeleted => LobbyEvent::DeckDeleted,
        Expect::Pool => serde_json::from_str::<PoolBody>(body).map_or_else(
            |_| unreadable(lang, Phrase::ThePool),
            |b| LobbyEvent::Pool {
                cards: b.cards,
                has_text: b.has_text,
            },
        ),
        Expect::Printings => serde_json::from_str::<PrintingsBody>(body).map_or_else(
            |_| unreadable(lang, Phrase::ThePrintings),
            |b| LobbyEvent::Printings {
                card: b.card,
                printings: b.printings,
                from_catalog: b.from_catalog,
            },
        ),
        Expect::DeckLoaded => serde_json::from_str::<StoredDeck>(body).map_or_else(
            |_| unreadable(lang, Phrase::TheDeck),
            |d| LobbyEvent::DeckLoaded {
                id: d.id,
                name: d.name,
                cards: d.cards,
                sideboard: d.sideboard,
                commanders: d.commanders,
            },
        ),
        Expect::LoggedIn => serde_json::from_str::<TokenBody>(body).map_or_else(
            |_| unreadable(lang, Phrase::TheSignIn),
            |b| LobbyEvent::LoggedIn { token: b.token },
        ),
        Expect::Decks => serde_json::from_str(body)
            .map_or_else(|_| unreadable(lang, Phrase::TheDeckList), LobbyEvent::Decks),
        Expect::Games => serde_json::from_str(body)
            .map_or_else(|_| unreadable(lang, Phrase::TheGameList), LobbyEvent::Games),
        // The body is the whole lobby, which is not what is being read.
        Expect::Moved => LobbyEvent::Moved,
        Expect::Seat => serde_json::from_str(body)
            .map_or_else(|_| unreadable(lang, Phrase::TheSeat), LobbyEvent::Seated),
        // Nothing comes back, so the lobby re-reads the list to find out what
        // the table looks like without us.
        Expect::Left => LobbyEvent::Left,
    }
}

/// A table query as a query string, ready to append to a URL.
///
/// Shared with the push socket, which asks for the same page over a different
/// transport — the search a player typed has to reach both or the socket
/// starts answering a different question than the button did.
pub(super) fn params(query: &GameQuery) -> String {
    format!(
        "q={}&offset={}&limit={}",
        escape(&query.q),
        query.offset,
        query.limit
    )
}

/// Percent-encodes one query-string value.
///
/// Small on purpose: a table search is a person's typing, so the set that has
/// to survive is "anything at all", and the set that must not pass through is
/// everything with a meaning in a URL.
pub(super) fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(*byte));
            }
            _ => {
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                out.push('%');
                out.push(char::from(HEX[usize::from(byte >> 4)]));
                out.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
        }
    }
    out
}

/// The message for a body that arrived but made no sense.
fn unreadable(lang: Lang, what: Phrase) -> LobbyEvent {
    LobbyEvent::Failed(Phrase::Unreadable.fill(lang, &[what.text(lang)]))
}

/// The gateway's own `{"error":…}`, or the bare status if it sent none.
pub(super) fn gateway_error(lang: Lang, response: &ehttp::Response) -> String {
    /// Every refusal the gateway sends has this shape.
    #[derive(serde::Deserialize)]
    struct Body {
        error: String,
    }

    response
        .text()
        .and_then(|body| serde_json::from_str::<Body>(body).ok())
        .map_or_else(
            || Phrase::GatewayAnswered.fill(lang, &[&response.status.to_string()]),
            |b| match b.error.as_str() {
                "invalid display name" => Phrase::AccountNameHint.text(lang).to_string(),
                "weak password" | "invalid password" => {
                    Phrase::AccountPasswordInvalid.text(lang).to_string()
                }
                _ => client_core::i18n::server_message(lang, &b.error),
            },
        )
}

/// Asks once, at startup, whether this gateway takes sign-ups — and whether it
/// mirrors card art.
pub(super) fn ask_about_registration(state: Res<LobbyState>, mailbox: Res<Mailbox>) {
    if state.gateway_selected {
        probe_registration(&state, &mailbox);
    }
}

/// `GET /auth/config`.
#[derive(serde::Deserialize)]
struct AuthConfig {
    registration_enabled: bool,
    /// Whether `GET /art/…` serves card images.
    ///
    /// Defaulted rather than required: a gateway built before the mirror
    /// existed answers without the field, and the right reading of a
    /// missing answer is "no mirror", which is exactly what the client
    /// already did.
    #[serde(default)]
    art_cache: bool,
}

pub(super) fn probe_registration(state: &LobbyState, mailbox: &Mailbox) {
    let box_ = Arc::clone(&mailbox.0);
    let gateway = state.gateway.clone();
    let epoch = state.gateway_epoch;
    let url = format!("{gateway}/auth/config");
    ehttp::fetch(ehttp::Request::get(&url), move |result| {
        let body = match result {
            Ok(response) if response.ok => response
                .text()
                .and_then(|body| serde_json::from_str::<AuthConfig>(body).ok()),
            // A gateway that is not up yet says nothing about registration.
            // Leaving the offer standing is the recoverable failure.
            _ => None,
        };
        let Some(body) = body else {
            return;
        };
        if let Ok(mut box_) = box_.lock() {
            box_.push(Reply::Remote(
                epoch,
                Box::new(Reply::Registration {
                    enabled: body.registration_enabled,
                    art_cache: body.art_cache,
                }),
            ));
        }
    });
}

/// Asks every saved address about itself, once, at startup.
pub(super) fn ask_about_saved_gateways(mut state: ResMut<LobbyState>, mailbox: Res<Mailbox>) {
    for url in state.unasked_gateways() {
        probe_gateway(url, &mailbox);
    }
}

/// How long asking an address about itself may take.
///
/// Shorter than a lobby request's thirty seconds, because saving waits on
/// it with the button held down, and a gateway that has not said what it is
/// in ten seconds is not one to play on.
pub(super) const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Asks an address what it is (`GET /info`), and leaves the answer in the
/// mailbox under the address.
///
/// A 404 is asked once more, of `GET /auth/config`: every gateway from
/// before `/info` answers that route, and a server that answers neither is
/// not a gateway. Without the second question a mistyped *path* on a real
/// web server would be saved as an old gateway.
pub(super) fn probe_gateway(url: String, mailbox: &Mailbox) {
    let box_ = Arc::clone(&mailbox.0);
    let (info, older) = probe_requests(&url);
    ehttp::fetch(info, move |answer| {
        if let Some(probe) = read_info(&answer) {
            post_probe(&box_, url, probe);
            return;
        }
        ehttp::fetch(older, move |answer| {
            post_probe(&box_, url, read_older(&answer));
        });
    });
}

/// The two questions [`probe_gateway`] may ask: `GET /info`, then
/// `GET /auth/config` when that route is missing.
///
/// Built here and not inline, so a test can read the paths. A typo in the
/// first one would not fail anything a player sees at once: every real
/// gateway would answer the second, and the whole list would quietly turn
/// "version unknown".
pub(super) fn probe_requests(url: &str) -> (ehttp::Request, ehttp::Request) {
    let url = url.trim_end_matches('/');
    (
        ehttp::Request::get(format!("{url}/info")).with_timeout(Some(PROBE_TIMEOUT)),
        ehttp::Request::get(format!("{url}/auth/config")).with_timeout(Some(PROBE_TIMEOUT)),
    )
}

fn post_probe(mailbox: &Mutex<Vec<Reply>>, url: String, probe: Probe) {
    if let Ok(mut mailbox) = mailbox.lock() {
        mailbox.push(Reply::Gateway { url, probe });
    }
}

/// What an answer to `GET /info` says, or `None` when the route is missing
/// and `GET /auth/config` has to be asked instead.
pub(super) fn read_info(answer: &Result<ehttp::Response, String>) -> Option<Probe> {
    match answer {
        Ok(response) if response.ok => Some(
            client_core::lobby::gateway_info::GatewayInfo::read(&response.bytes)
                .map_or(Probe::Silent, Probe::Known),
        ),
        Ok(response) if response.status == 404 => None,
        _ => Some(Probe::Silent),
    }
}

/// What an answer to `GET /auth/config` says about an address with no
/// `GET /info`.
pub(super) fn read_older(answer: &Result<ehttp::Response, String>) -> Probe {
    match answer {
        Ok(response)
            if response.ok && serde_json::from_slice::<AuthConfig>(&response.bytes).is_ok() =>
        {
            Probe::Older
        }
        _ => Probe::Silent,
    }
}

fn library_request(
    base: &str,
    request: client_core::lobby::library::Request,
) -> (ehttp::Request, Expect) {
    use client_core::lobby::library::Request;
    let http = match &request {
        Request::House => ehttp::Request::get(format!("{base}/decks/shared")),
        Request::History(id) => ehttp::Request::get(format!("{base}/decks/{id}/history")),
        Request::Version(id, version) => {
            ehttp::Request::get(format!("{base}/decks/{id}/versions/{version}"))
        }
        Request::Copy(id) => json_post(&format!("{base}/decks/{id}/copy"), &serde_json::json!({})),
        Request::Restore(id, version) => json_post(
            &format!("{base}/decks/{id}/versions/{version}/revert"),
            &serde_json::json!({}),
        ),
    };
    (http, Expect::Library(request))
}

pub(super) fn decode_library(
    request: client_core::lobby::library::Request,
    body: &str,
    lang: Lang,
) -> LobbyEvent {
    use client_core::lobby::library::{Reply, Request};
    let parsed = match request {
        Request::House => serde_json::from_str(body).map(Reply::House),
        Request::History(id) => {
            serde_json::from_str(body).map(|history| Reply::History(id, history))
        }
        Request::Version(id, _) => {
            serde_json::from_str(body).map(|snapshot| Reply::Version(id, snapshot))
        }
        Request::Copy(_) => {
            #[derive(serde::Deserialize)]
            struct Copy {
                deck_id: String,
            }
            serde_json::from_str::<Copy>(body).map(|copy| Reply::Copied(copy.deck_id))
        }
        Request::Restore(id, _) => {
            serde_json::from_str::<serde_json::Value>(body).map(|_| Reply::Restored(id))
        }
    };
    LobbyEvent::Library(
        parsed.unwrap_or_else(|_| Reply::Failed(Phrase::LibraryReadFailed.text(lang).to_string())),
    )
}
