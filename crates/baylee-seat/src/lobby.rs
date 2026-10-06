//! The gateway's front door, as the bridge walks through it: a guest
//! account under the mind's name, a deck, a chair in a room, ready; or, for
//! a bridge its host's client started, the host's chair ticket, which sits
//! it down with its deck in one call and no account at all
//! ([`Lobby::redeem`]).
//!
//! Every call is an ordinary request a person's client makes, with the
//! session (or the chair ticket, or the seat token) in `Authorization` and
//! never in an address. A blocking HTTP
//! client on a blocking thread: the lobby is a handful of requests before a
//! game and one after a dropped socket, and the socket is the only thing
//! that has to be asynchronous.

use crate::deck::Deck;
use crate::mind::Disclosure;
use anyhow::{Context as _, anyhow, bail};
use serde::Deserialize;
use std::time::Duration;

/// A gateway, by its HTTP address (`http://…` or `https://…`).
#[derive(Clone)]
pub struct Lobby {
    base: String,
    agent: ureq::Agent,
}

/// A chair taken: its number and the secret its socket is opened with.
#[derive(Clone)]
pub struct Chair {
    /// The game the chair is at.
    pub game_id: String,
    /// The chair's number.
    pub seat: usize,
    /// The seat token. Never logged, never in an address.
    pub seat_token: String,
}

impl std::fmt::Debug for Chair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chair")
            .field("game_id", &self.game_id)
            .field("seat", &self.seat)
            .finish_non_exhaustive()
    }
}

/// A room as the listing shows it: what the bridge checks before it sits.
#[derive(Clone, Debug, Deserialize)]
pub struct Room {
    /// The room's id, which is the game's.
    pub id: String,
    /// `waiting`, `playing` or `over`.
    pub state: String,
    /// Whether the room asks for a password.
    #[serde(default)]
    pub locked: bool,
    /// The table's pace.
    #[serde(default)]
    pub clock: RoomClock,
    /// The chairs.
    #[serde(default)]
    pub seats: Vec<RoomSeat>,
}

/// The pace a room plays at.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct RoomClock {
    /// Seconds a question gets; `None` for an untimed table.
    #[serde(default)]
    pub decide_secs: Option<u32>,
}

/// One chair as the listing shows it.
#[derive(Clone, Debug, Deserialize)]
pub struct RoomSeat {
    /// The chair's number.
    pub seat: usize,
    /// `human` or `ai`.
    #[serde(default)]
    pub kind: String,
    /// Whether somebody sits in it.
    #[serde(default)]
    pub taken: bool,
    /// Whether it is the caller's.
    #[serde(default)]
    pub you: bool,
    /// Whether its player said ready.
    #[serde(default)]
    pub ready: bool,
}

impl Room {
    /// Whether the room is still being set up.
    #[must_use]
    pub fn waiting(&self) -> bool {
        self.state == "waiting"
    }

    /// Whether the game is on.
    #[must_use]
    pub fn playing(&self) -> bool {
        self.state == "playing"
    }

    /// Whether a person could sit down in it now.
    #[must_use]
    pub fn has_a_free_chair(&self) -> bool {
        self.seats
            .iter()
            .any(|seat| seat.kind == "human" && !seat.taken)
    }
}

/// What a guest signs in with.
#[derive(Clone, Debug, Default)]
pub struct GuestSignIn {
    /// The name other players see, with the mind's prefix.
    pub display_name: String,
    /// The closed-beta key, for a gateway that asks for one.
    pub invite_key: Option<String>,
}

/// A signed-in session. The token is never logged.
#[derive(Clone)]
pub struct Session {
    token: String,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Session(..)")
    }
}

impl Session {
    /// A session from a token handed over by something that signed in.
    #[must_use]
    pub const fn from_token(token: String) -> Self {
        Self { token }
    }
}

/// A chair ticket: the host's word that this bridge may sit in one chair of
/// its room (`docs/protocol.md` §"A host's chair for a seat bridge"). Read
/// from the bridge's stdin, shown to the gateway once as `Authorization`,
/// and never printed, logged or put in an address: its `Debug` says only
/// that there is one.
#[derive(Clone)]
pub struct ChairTicket(String);

impl std::fmt::Debug for ChairTicket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ChairTicket(..)")
    }
}

impl ChairTicket {
    /// The longest line read as a ticket. The gateway's are 64 hex digits.
    pub const MAX_LEN: usize = 128;

    /// The ticket on `line` (one line of stdin, its end trimmed).
    ///
    /// # Errors
    /// When the line is empty, too long, or holds anything but letters and
    /// digits; the error never quotes it.
    pub fn read(line: &str) -> anyhow::Result<Self> {
        let ticket = line.trim();
        if ticket.is_empty() {
            bail!("no chair ticket on stdin");
        }
        if ticket.len() > Self::MAX_LEN || !ticket.bytes().all(|b| b.is_ascii_alphanumeric()) {
            bail!("the line on stdin is not a chair ticket");
        }
        Ok(Self(ticket.to_string()))
    }
}

/// What the gateway says of a chair to the bridge that holds it
/// (`GET /lobby/games/{id}/chair`).
#[derive(Clone, Debug, Deserialize)]
pub struct ChairStatus {
    /// `waiting`, `playing` or `over`.
    pub state: String,
    /// Seconds a question gets; `None` for an untimed table.
    #[serde(default)]
    pub decide_secs: Option<u32>,
}

/// The name a mind sits under: `prefix` + `name`, held to the gateway's
/// display-name rule (3 to 16 of `A-Z a-z 0-9 _ -`, a letter or digit at
/// each end).
///
/// # Errors
/// When the whole name breaks that rule.
pub fn seat_name(disclosure: Disclosure, name: &str) -> anyhow::Result<String> {
    let whole = format!("{}{name}", disclosure.prefix());
    let plain = |c: Option<char>| c.is_some_and(|c| c.is_ascii_alphanumeric());
    let fits = (3..=16).contains(&whole.len())
        && plain(whole.chars().next())
        && plain(whole.chars().next_back())
        && whole
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if !fits {
        bail!(
            "«{whole}» is not a name the gateway takes: 3 to 16 letters, digits, `_` or `-`, \
             a letter or digit at each end"
        );
    }
    Ok(whole)
}

impl Lobby {
    /// The gateway at `base` (`http://host:port`, no trailing slash needed).
    #[must_use]
    pub fn new(base: &str) -> Self {
        // No redirect is followed ([`Lobby::raw`] fails a 3xx): what the
        // gateway is asked is asked of the gateway named, and of no address
        // a `Location` names.
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .new_agent();
        Self {
            base: base.trim_end_matches('/').to_string(),
            agent,
        }
    }

    /// The gateway's HTTP address.
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    /// A guest account and its session (`POST /auth/guest`).
    ///
    /// # Errors
    /// When the gateway takes no guests, the name, or the key.
    pub async fn guest(&self, sign_in: &GuestSignIn) -> anyhow::Result<Session> {
        let body = serde_json::json!({
            "display_name": sign_in.display_name,
            "lang": "en",
            "invite_key": sign_in.invite_key,
        });
        let answer = self.call("POST", "/auth/guest", None, Some(body)).await?;
        let token = answer.expect_ok("sign in as a guest")?.string("token")?;
        Ok(Session { token })
    }

    /// Stores `deck` under the session (`POST /decks`): its id.
    ///
    /// # Errors
    /// When the gateway refuses the deck.
    pub async fn upload(&self, session: &Session, deck: &Deck) -> anyhow::Result<String> {
        let answer = self
            .call("POST", "/decks", Some(&session.token), Some(deck.upload()))
            .await?;
        answer
            .expect_ok(&format!("store the deck {}", deck.name))?
            .string("deck_id")
    }

    /// The room `game_id`, as the listing shows it to this session.
    ///
    /// # Errors
    /// When the listing cannot be read.
    pub async fn room(&self, session: &Session, game_id: &str) -> anyhow::Result<Option<Room>> {
        #[derive(Deserialize)]
        struct Page {
            games: Vec<Room>,
        }
        let path = format!("/lobby/games?q={}&limit=50", escape(game_id));
        let answer = self.call("GET", &path, Some(&session.token), None).await?;
        let page: Page = serde_json::from_value(answer.expect_ok("read the lobby")?.body)
            .context("an unreadable lobby listing")?;
        Ok(page.games.into_iter().find(|room| room.id == game_id))
    }

    /// Opens a room with `seats` chairs and takes the first
    /// (`POST /lobby/games`): the chair.
    ///
    /// # Errors
    /// When the gateway refuses to open it.
    pub async fn open(
        &self,
        session: &Session,
        deck_id: &str,
        seats: usize,
        name: &str,
    ) -> anyhow::Result<Chair> {
        let body = serde_json::json!({ "deck_id": deck_id, "seats": seats, "name": name });
        let answer = self
            .call("POST", "/lobby/games", Some(&session.token), Some(body))
            .await?
            .expect_ok("open a room")?;
        Ok(Chair {
            game_id: answer.string("game_id")?,
            seat: answer.number("seat")?,
            seat_token: answer.string("seat_token")?,
        })
    }

    /// Takes a free chair in the room with the deck (`POST …/join`): the
    /// one `seat` names, else the first free one.
    ///
    /// # Errors
    /// When the room is full, started, locked or gone, or the chair named
    /// is taken.
    pub async fn join(
        &self,
        session: &Session,
        game_id: &str,
        deck_id: &str,
        password: Option<&str>,
        seat: Option<u32>,
    ) -> anyhow::Result<Chair> {
        let mut body = serde_json::json!({ "deck_id": deck_id, "password": password });
        if let Some(seat) = seat {
            body["seat"] = serde_json::json!(seat);
        }
        let path = format!("/lobby/games/{}/join", escape(game_id));
        let answer = self
            .call("POST", &path, Some(&session.token), Some(body))
            .await?
            .expect_ok("take a chair")?;
        Ok(Chair {
            game_id: game_id.to_string(),
            seat: answer.number("seat")?,
            seat_token: answer.string("seat_token")?,
        })
    }

    /// Hands this session its chair back with a new seat token
    /// (`POST …/seat`), for a seat token the table no longer takes.
    ///
    /// # Errors
    /// When the session is not at the table or the game is over.
    pub async fn take_back(&self, session: &Session, game_id: &str) -> anyhow::Result<Chair> {
        let path = format!("/lobby/games/{}/seat", escape(game_id));
        let answer = self
            .call(
                "POST",
                &path,
                Some(&session.token),
                Some(serde_json::json!({})),
            )
            .await?
            .expect_ok("take the chair back")?;
        Ok(Chair {
            game_id: game_id.to_string(),
            seat: answer.number("seat")?,
            seat_token: answer.string("seat_token")?,
        })
    }

    /// Says the chair is ready (`POST …/ready`).
    ///
    /// # Errors
    /// When the room refuses it.
    pub async fn ready(&self, session: &Session, game_id: &str) -> anyhow::Result<()> {
        let path = format!("/lobby/games/{}/ready", escape(game_id));
        self.call(
            "POST",
            &path,
            Some(&session.token),
            Some(serde_json::json!({ "ready": true })),
        )
        .await?
        .expect_ok("say ready")?;
        Ok(())
    }

    /// Starts the room, as its host (`POST …/start`).
    ///
    /// # Errors
    /// When not everyone is ready or the session is not the host.
    pub async fn start(&self, session: &Session, game_id: &str) -> anyhow::Result<()> {
        let path = format!("/lobby/games/{}/start", escape(game_id));
        self.call(
            "POST",
            &path,
            Some(&session.token),
            Some(serde_json::json!({})),
        )
        .await?
        .expect_ok("start the room")?;
        Ok(())
    }

    /// Gives the chair up (`POST …/leave`).
    ///
    /// # Errors
    /// When the gateway cannot be reached.
    pub async fn leave(&self, session: &Session, game_id: &str) -> anyhow::Result<()> {
        let path = format!("/lobby/games/{}/leave", escape(game_id));
        self.call(
            "POST",
            &path,
            Some(&session.token),
            Some(serde_json::json!({})),
        )
        .await?
        .expect_ok("leave the room")?;
        Ok(())
    }

    /// Waits, looking every `every`, until the room's game is on, saying
    /// ready again whenever the room lists this session's chair as not
    /// ready: a host rearranging the table takes every player's yes back,
    /// and a bridge waiting on it would otherwise keep the room from ever
    /// starting (beta.5: "an LLM chair never shows Ready").
    ///
    /// # Errors
    /// When the room is gone or over before it starts, or refuses the yes.
    pub async fn wait_for_start(
        &self,
        session: &Session,
        game_id: &str,
        every: Duration,
    ) -> anyhow::Result<()> {
        loop {
            match self.room(session, game_id).await? {
                Some(room) if room.playing() => return Ok(()),
                Some(room) if room.waiting() => {
                    if room.seats.iter().any(|seat| seat.you && !seat.ready) {
                        self.ready(session, game_id).await?;
                    }
                    tokio::time::sleep(every).await;
                }
                Some(room) => bail!("the room {game_id} is {}", room.state),
                None => bail!("the room {game_id} is gone"),
            }
        }
    }

    /// Sits down in chair `seat` of room `game_id` on its host's `ticket`,
    /// under `display_name` with `deck` (`POST …/chairs/{seat}/redeem`):
    /// the chair, and the seconds its table gives a question. No session:
    /// the host vouched for the chair, so a gateway that takes no guests
    /// seats the bridge all the same.
    ///
    /// # Errors
    /// When the ticket is refused (used, expired, for another chair, or its
    /// host gone) or the chair is no longer open; never quoting the ticket.
    pub async fn redeem(
        &self,
        ticket: &ChairTicket,
        game_id: &str,
        seat: u32,
        display_name: &str,
        deck: &Deck,
    ) -> anyhow::Result<(Chair, Option<u32>)> {
        let body = serde_json::json!({ "display_name": display_name, "deck": deck.upload() });
        let path = baylee_protocol::chair_redeem_path(&escape(game_id), seat);
        let answer = self
            .call("POST", &path, Some(&ticket.0), Some(body))
            .await?
            .expect_ok("sit down on the host's chair ticket")?;
        let decide_secs = answer
            .body
            .get("decide_secs")
            .and_then(serde_json::Value::as_u64)
            .and_then(|n| u32::try_from(n).ok());
        Ok((
            Chair {
                game_id: game_id.to_string(),
                seat: answer.number("seat")?,
                seat_token: answer.string("seat_token")?,
            },
            decide_secs,
        ))
    }

    /// The chair's room as the gateway tells the chair's holder
    /// (`GET …/chair`, the seat token as `Authorization`); `None` when the
    /// room is gone or no longer takes the token (the host took the chair
    /// back, or left).
    ///
    /// # Errors
    /// When the gateway cannot be reached or answers otherwise.
    pub async fn chair_status(&self, chair: &Chair) -> anyhow::Result<Option<ChairStatus>> {
        let path = format!("/lobby/games/{}/chair", escape(&chair.game_id));
        let answer = self
            .call("GET", &path, Some(&chair.seat_token), None)
            .await?;
        if matches!(answer.status, 401 | 404) {
            return Ok(None);
        }
        let status = answer.expect_ok("ask after the chair")?.body;
        Ok(Some(
            serde_json::from_value(status).context("an unreadable chair status")?,
        ))
    }

    /// Says a chair taken on a host's ticket may play (`POST …/chair/ready`,
    /// the seat token as `Authorization`): its mind answered its check.
    ///
    /// # Errors
    /// When the gateway cannot be reached or refuses.
    pub async fn chair_ready(&self, chair: &Chair) -> anyhow::Result<()> {
        let path = format!("/lobby/games/{}/chair/ready", escape(&chair.game_id));
        self.call(
            "POST",
            &path,
            Some(&chair.seat_token),
            Some(serde_json::json!({ "ready": true })),
        )
        .await?
        .expect_ok("say the chair is ready")?;
        Ok(())
    }

    /// Gives a chair taken on a host's ticket back before the game
    /// (`POST …/chair/leave`, the seat token as `Authorization`).
    ///
    /// # Errors
    /// When the gateway cannot be reached or refuses.
    pub async fn leave_chair(&self, chair: &Chair) -> anyhow::Result<()> {
        let path = format!("/lobby/games/{}/chair/leave", escape(&chair.game_id));
        self.call(
            "POST",
            &path,
            Some(&chair.seat_token),
            Some(serde_json::json!({})),
        )
        .await?
        .expect_ok("leave the chair")?;
        Ok(())
    }

    /// [`Lobby::wait_for_start`] for a chair taken on a host's ticket,
    /// asked with its seat token.
    ///
    /// # Errors
    /// When the room is gone, over, or no longer takes the chair before it
    /// starts.
    pub async fn wait_for_chair_start(&self, chair: &Chair, every: Duration) -> anyhow::Result<()> {
        loop {
            match self.chair_status(chair).await? {
                Some(status) if status.state == "playing" => return Ok(()),
                Some(status) if status.state == "waiting" => tokio::time::sleep(every).await,
                Some(status) => bail!("the room {} is {}", chair.game_id, status.state),
                None => bail!(
                    "the room {} is gone, or its host took the chair back",
                    chair.game_id
                ),
            }
        }
    }

    /// A ticket for one upgrade of the seat's socket (`POST /ws-ticket`),
    /// read by the client's rule.
    pub async fn seat_ticket(&self, chair: &Chair) -> baylee_client_core::wsticket::TicketAnswer {
        use baylee_client_core::wsticket::{Socket, TicketAnswer};
        let body = Socket::Seat {
            game_id: &chair.game_id,
        }
        .request_body();
        let request = self.raw(
            "POST",
            baylee_protocol::WS_TICKET_PATH,
            Some(chair.seat_token.clone()),
            Some(body.into_bytes()),
        );
        match request.await {
            Ok((status, body)) => TicketAnswer::read(status, &body),
            Err(e) => TicketAnswer::Failed(format!("{e:#}")),
        }
    }

    async fn call(
        &self,
        method: &'static str,
        path: &str,
        bearer: Option<&str>,
        body: Option<serde_json::Value>,
    ) -> anyhow::Result<Answer> {
        let bytes = body.map(|body| body.to_string().into_bytes());
        let (status, body) = self
            .raw(method, path, bearer.map(ToString::to_string), bytes)
            .await?;
        let body = if body.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null)
        };
        Ok(Answer { status, body })
    }

    /// One request on a blocking thread: the status and the body.
    async fn raw(
        &self,
        method: &'static str,
        path: &str,
        bearer: Option<String>,
        body: Option<Vec<u8>>,
    ) -> anyhow::Result<(u16, Vec<u8>)> {
        let url = format!("{}{path}", self.base);
        let agent = self.agent.clone();
        let shown = format!("{method} {}", path.split('?').next().unwrap_or(path));
        tokio::task::spawn_blocking(move || -> anyhow::Result<(u16, Vec<u8>)> {
            let mut request = ureq::http::Request::builder().method(method).uri(&url);
            if let Some(bearer) = bearer {
                request = request.header("authorization", format!("Bearer {bearer}"));
            }
            let mut answer = match body {
                Some(body) => agent.run(
                    request
                        .header("content-type", "application/json")
                        .body(body)?,
                )?,
                None => agent.run(request.body(())?)?,
            };
            let status = answer.status().as_u16();
            // By its status alone, neither where it pointed nor its body.
            if answer.status().is_redirection() {
                bail!("the gateway answered {status}, a redirect, and the seat follows none");
            }
            let body = answer.body_mut().read_to_vec()?;
            Ok((status, body))
        })
        .await
        .map_err(|e| anyhow!("{shown}: {e}"))?
        .with_context(|| shown.clone())
    }
}

/// A gateway's answer, read as JSON.
struct Answer {
    status: u16,
    body: serde_json::Value,
}

impl Answer {
    /// The answer when it is a success, or the gateway's sentence for why
    /// not, with what was being done.
    fn expect_ok(self, doing: &str) -> anyhow::Result<Self> {
        if (200..300).contains(&self.status) {
            return Ok(self);
        }
        let why = self
            .body
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("no reason given");
        bail!("{doing}: the gateway answered {} ({why})", self.status)
    }

    fn string(&self, field: &str) -> anyhow::Result<String> {
        self.body
            .get(field)
            .and_then(serde_json::Value::as_str)
            .map(ToString::to_string)
            .ok_or_else(|| anyhow!("the gateway's answer has no `{field}`"))
    }

    fn number(&self, field: &str) -> anyhow::Result<usize> {
        self.body
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| anyhow!("the gateway's answer has no `{field}`"))
    }
}

/// A room id as a path segment or a query value: ids are the gateway's own
/// and plain, but one typed at a command line need not be.
fn escape(text: &str) -> String {
    text.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                char::from(b).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_seat_name_is_the_prefix_and_a_name_the_gateway_takes() {
        assert_eq!(seat_name(Disclosure::Llm, "claude").unwrap(), "LLM-claude");
        assert_eq!(seat_name(Disclosure::Net, "n1").unwrap(), "NET-n1");
        assert_eq!(
            seat_name(Disclosure::House, "house").unwrap(),
            "HOUSE-house"
        );
        assert_eq!(
            seat_name(Disclosure::Scripted, "scripted").unwrap(),
            "TEST-scripted"
        );
        assert!(
            seat_name(Disclosure::Llm, "").is_err(),
            "a prefix is not a name"
        );
        assert!(
            seat_name(Disclosure::Llm, "a-very-long-one").is_err(),
            "over 16"
        );
        assert!(seat_name(Disclosure::Llm, "two words").is_err());
        assert!(
            seat_name(Disclosure::Llm, "x-").is_err(),
            "a dash at the end"
        );
        // What the name is checked against at the table, too: each kind's
        // name is that kind's and no other's.
        for kind in Disclosure::ALL {
            let name = seat_name(kind, "x1").unwrap();
            for other in Disclosure::ALL {
                assert_eq!(
                    other.names(&name),
                    other == kind,
                    "«{name}» read as {other:?}"
                );
            }
        }
    }

    /// A gateway that answers with a redirect is followed nowhere: every
    /// call fails by the status alone, and the session goes to no other
    /// address.
    #[tokio::test]
    async fn a_redirect_is_not_followed() {
        let secret = "TEST-session-0123456789abcdef";
        let session = Session {
            token: secret.to_string(),
        };
        let moved = serde_json::json!({"error": "see LOCATION"});
        let deck = Deck::acceptance("Victory").unwrap();
        for status in [301, 302, 303, 307, 308] {
            let redirect = crate::testnet::redirect(status, &moved).await;
            let lobby = Lobby::new(&redirect.base);
            assert_eq!(lobby.agent.config().max_redirects(), 0);
            let posted = lobby.upload(&session, &deck).await.expect_err("a POST");
            let got = lobby.room(&session, "g1").await.expect_err("a GET");
            for error in [posted, got] {
                let said = format!("{error:#}");
                assert!(said.contains(&status.to_string()), "{said}");
                assert!(said.contains("redirect"), "{said}");
                assert!(!said.contains(secret), "the session: {said}");
                assert!(
                    !said.contains(&redirect.target_port),
                    "the location: {said}"
                );
                assert!(!said.contains("see "), "the body: {said}");
            }
            let chair = Chair {
                game_id: "g1".into(),
                seat: 0,
                seat_token: secret.into(),
            };
            let ticket = lobby.seat_ticket(&chair).await;
            assert!(
                matches!(&ticket, baylee_client_core::wsticket::TicketAnswer::Failed(why)
                    if why.contains(&status.to_string()) && !why.contains(secret)),
                "{ticket:?}"
            );
            assert_eq!(redirect.asked().len(), 3, "each call reached the gateway");
            assert_eq!(redirect.followed(), 0, "{status} was followed");
        }
    }

    /// A chair ticket is read off one line and is never shown again: not by
    /// `Debug`, and not by the sentence a bad line is refused with.
    #[test]
    fn a_chair_ticket_is_read_from_a_line_and_never_shown() {
        let secret = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let ticket = ChairTicket::read(&format!("{secret}\n")).expect("a ticket");
        assert_eq!(ticket.0, secret);
        assert_eq!(format!("{ticket:?}"), "ChairTicket(..)");
        assert!(ChairTicket::read("\n").is_err(), "an empty line");
        let long = "a".repeat(ChairTicket::MAX_LEN + 1);
        for bad in [format!("{secret} extra"), format!("{secret};"), long] {
            let said = format!("{:#}", ChairTicket::read(&bad).expect_err("refused"));
            assert!(!said.contains(secret), "{said}");
            assert!(!said.contains("aaaa"), "{said}");
        }
    }

    /// Every call made on a chair ticket fails by its status alone, and the
    /// ticket goes to no other address and into no error.
    #[tokio::test]
    async fn a_chair_ticket_is_not_followed_by_a_redirect_nor_quoted() {
        let secret = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
        let ticket = ChairTicket::read(secret).unwrap();
        let moved = serde_json::json!({"error": "see LOCATION"});
        let deck = Deck::acceptance("Victory").unwrap();
        let redirect = crate::testnet::redirect(307, &moved).await;
        let lobby = Lobby::new(&redirect.base);
        let said = format!(
            "{:#}",
            lobby
                .redeem(&ticket, "g1", 1, "LLM-test", &deck)
                .await
                .expect_err("a redirect")
        );
        assert!(said.contains("redirect"), "{said}");
        assert!(!said.contains(secret), "the ticket: {said}");
        assert_eq!(redirect.followed(), 0);
    }

    #[test]
    fn an_id_is_escaped_for_an_address() {
        assert_eq!(escape("0193-abc_d"), "0193-abc_d");
        assert_eq!(escape("a b/c?"), "a%20b%2Fc%3F");
    }
}
