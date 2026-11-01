//! The seat's socket, dialled by the client's rules.
//!
//! Every dial buys a fresh ticket with the seat token (`POST /ws-ticket`)
//! and opens `/games/{id}/ws?ticket=…&protocol=…`
//! ([`baylee_protocol::seat_socket_path`]); the token never rides in an
//! address. An upgrade refused before the socket opened is most likely a
//! ticket that expired or was spent, and is tried again with a fresh one
//! ([`TicketDial`]); a table that cannot be reached is dialled again on the
//! client's back-off ([`Retry`]), which is also what covers a seat socket
//! opened while the game's engine is still starting. A seat token the table
//! no longer takes is traded for a new one with the session, as a client
//! that restarted takes its chair back.
//!
//! A room that closed is not dialled again. A finished game's seat socket
//! still opens and is then held without a frame until the gateway gives up
//! on its engine, so a dial alone never learns the game is gone: the lobby
//! is asked ([`SeatLink::room_closed`]) whenever a dial fails.
//!
//! A chair taken on its host's ticket has no session to trade a refused
//! token with, and the gateway answers `401` both when it will not have the
//! chair any more (the host took it back, the host's sign-in ended) and when
//! it could not check (a failed Central check). Such a chair leaves only
//! once the refusal has lasted [`REFUSAL_PATIENCE`]; until then it is asked
//! after and dialled again like a table that did not answer. Before
//! 22.10.2026 the first refusal ended the seat for good, and with the 5-s
//! check behind every socket a host's LLM seat "keeps losing its
//! connection" (owner).

use crate::lobby::{Chair, ChairAnswer, Lobby, Session};
use anyhow::{anyhow, bail};
use baylee_client_core::reconnect::{Retry, Window};
use baylee_client_core::wsticket::{Step as DialStep, TicketDial};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::Instant;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// An open seat socket.
pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// How a seat files its game's bug report ([`SeatLink::reporter`]).
#[derive(Clone, Copy)]
pub(crate) enum Reporter<'a> {
    /// Under its own session, as a player does (`POST /reports`).
    Session(&'a Lobby, &'a Session),
    /// On its chair's seat token, the chair taken on its host's ticket
    /// (`POST /lobby/games/{id}/chair/report`): the gateway files it under
    /// the host's pseudonym.
    Chair(&'a Lobby, &'a Chair),
}

/// How long a chair taken on its host's ticket goes on being refused (`401`)
/// before it counts its room as closed to it.
///
/// Long enough to ride over a check the gateway could not make for a while:
/// it checks every socket every five seconds and answers a failed check
/// `401`, so this spans nine of them. Short enough that a chair its host took
/// back, or whose host's sign-in ended, does not linger long: well inside a
/// table's window (180 s by default), after which the house plays the seat.
pub const REFUSAL_PATIENCE: Duration = Duration::from_secs(45);

/// Why one dial did not open.
#[derive(Debug)]
enum Missed {
    /// The table does not take this seat's token, and it could not be
    /// traded for one it takes.
    Refused(String),
    /// The table did not take the token of a chair taken on its host's
    /// ticket, which has nothing to trade it with: dialled again like
    /// [`Missed::Unreachable`] while [`SeatLink::room_closed`] says the room
    /// is still open to it.
    NotAdmitted,
    /// Nothing answered, or it answered with something else: worth another
    /// dial later.
    Unreachable(String),
}

/// One seat's way to its table.
pub struct SeatLink {
    lobby: Lobby,
    chair: Chair,
    session: Option<Session>,
    dials: u32,
    /// Since when the gateway has refused this chair's token, without a
    /// word from it since that took it.
    refused_since: Option<Instant>,
    /// [`REFUSAL_PATIENCE`], or a test's.
    patience: Duration,
    /// The table's reconnect window, once `GameStatic` has said it.
    window: Window,
}

impl SeatLink {
    /// The link to `chair`'s table; `session`, when given, takes the chair
    /// back if its seat token stops opening it.
    #[must_use]
    pub const fn new(lobby: Lobby, chair: Chair, session: Option<Session>) -> Self {
        Self {
            lobby,
            chair,
            session,
            dials: 0,
            refused_since: None,
            patience: REFUSAL_PATIENCE,
            window: Window::Unknown,
        }
    }

    /// The same link, leaving after `patience` of refusals rather than
    /// [`REFUSAL_PATIENCE`]: for a test that cannot wait that long.
    #[must_use]
    pub const fn with_refusal_patience(mut self, patience: Duration) -> Self {
        self.patience = patience;
        self
    }

    /// Holds reconnection open at least until the table's window has passed.
    pub const fn hold_for(&mut self, window: Window) {
        self.window = window;
    }

    /// The chair this link opens.
    #[must_use]
    pub const fn chair(&self) -> &Chair {
        &self.chair
    }

    /// The deck handed to a delegated bridge after the room starts.
    /// A session-backed player already brought its own deck.
    pub(crate) async fn selected_deck(&self) -> anyhow::Result<Option<crate::mind::DeckList>> {
        if self.session.is_some() {
            return Ok(None);
        }
        let Some(mut deck) = self
            .lobby
            .chair_status(&self.chair)
            .await?
            .and_then(|status| status.deck)
        else {
            return Ok(None);
        };
        deck.resolve()?;
        Ok(Some(deck.list))
    }

    /// Where the game's reports are filed, and with what: the seat's own
    /// session where it has one, else, for a chair taken on its host's
    /// ticket, the chair's seat token.
    pub(crate) const fn reporter(&self) -> Reporter<'_> {
        match &self.session {
            Some(session) => Reporter::Session(&self.lobby, session),
            None => Reporter::Chair(&self.lobby, &self.chair),
        }
    }

    /// Sockets opened so far.
    #[must_use]
    pub const fn dials(&self) -> u32 {
        self.dials
    }

    /// Opens the socket, dialling again on the client's back-off until it
    /// opens or the schedule gives up; `None` when the room closed.
    ///
    /// # Errors
    /// When the seat's token is refused and cannot be traded with the
    /// session for one it takes, or after [`Retry::GIVE_UP`] dials and, once
    /// known, the table's window. A chair taken on its host's ticket ends on
    /// a refusal only through [`SeatLink::room_closed`].
    pub async fn connect(&mut self) -> anyhow::Result<Option<Socket>> {
        let mut retry = Retry::new();
        retry.hold_for(self.window);
        let mut since = Instant::now();
        loop {
            match self.dial().await {
                Ok(socket) => {
                    self.dials += 1;
                    self.refused_since = None;
                    return Ok(Some(socket));
                }
                Err(_) if self.room_closed().await => return Ok(None),
                Err(Missed::Refused(why)) => bail!("the table refused this seat: {why}"),
                Err(miss) => {
                    let why = match miss {
                        Missed::Unreachable(why) | Missed::Refused(why) => why,
                        Missed::NotAdmitted => "the table did not take the seat's token".to_owned(),
                    };
                    let now = Instant::now();
                    retry.stayed_down(now.duration_since(since).as_secs_f32());
                    since = now;
                    if retry.exhausted() {
                        bail!(
                            "the table could not be reached after {} dials: {why}",
                            retry.attempts() + 1
                        );
                    }
                    tracing::info!(%why, wait_s = retry.wait(), "the seat socket did not open");
                    let wait = retry.wait();
                    tokio::time::sleep(Duration::from_secs_f32(wait)).await;
                    retry.tick(wait);
                }
            }
        }
    }

    /// Whether the room has closed: the lobby no longer lists it (it lists
    /// no finished room), or lists it as anything but playing.
    ///
    /// Without a session (a chair taken on its host's ticket) the chair
    /// itself is asked, with its seat token: a room that is gone, or no
    /// longer takes the token, is closed to it.
    ///
    /// `false` when that cannot be told (a lobby that did not answer): a
    /// seat does not leave a table on a guess. Nor on one refusal of a chair
    /// taken on its host's ticket: only once refusals have lasted the
    /// link's patience ([`REFUSAL_PATIENCE`]).
    pub async fn room_closed(&mut self) -> bool {
        let closed = match self.session.as_ref() {
            Some(session) => self
                .lobby
                .room(session, &self.chair.game_id)
                .await
                .map(|room| room.is_none_or(|room| !room.playing())),
            None => match self.lobby.chair(&self.chair).await {
                Ok(ChairAnswer::Status(status)) => {
                    self.refused_since = None;
                    Ok(status.state != "playing")
                }
                Ok(ChairAnswer::Gone) => Ok(true),
                Ok(ChairAnswer::Refused) => Ok(self.refused_too_long()),
                Err(e) => Err(e),
            },
        };
        match closed {
            Ok(closed) => closed,
            Err(e) => {
                tracing::info!(
                    error = %format!("{e:#}"),
                    "the lobby could not say whether the room is open"
                );
                false
            }
        }
    }

    /// One more refusal of this chair's token; whether they have lasted the
    /// link's patience, and the room is to be counted closed to it.
    fn refused_too_long(&mut self) -> bool {
        let since = *self.refused_since.get_or_insert_with(Instant::now);
        let lasted = since.elapsed();
        if lasted < self.patience {
            tracing::info!(
                refused_s = lasted.as_secs(),
                "the gateway refused this chair's token; asking again"
            );
            return false;
        }
        tracing::warn!(
            refused_s = lasted.as_secs(),
            "the gateway has refused this chair's token all this time: \
             the host took the chair back, or the host's sign-in ended"
        );
        true
    }

    /// One dial, from the first ticket to an open socket or a refusal.
    async fn dial(&mut self) -> Result<Socket, Missed> {
        let base = ws_base(self.lobby.base()).map_err(|e| Missed::Refused(e.to_string()))?;
        let (mut dial, mut step) = TicketDial::start();
        let mut retaken = false;
        let mut why = String::from("no dial was made");
        loop {
            step = match step {
                DialStep::FetchTicket => dial.answered(self.lobby.seat_ticket(&self.chair).await),
                DialStep::Dial(ticket) => {
                    let path = baylee_protocol::seat_socket_path(&self.chair.game_id, &ticket);
                    match open(&format!("{base}{path}")).await {
                        Ok(socket) => {
                            dial.opened();
                            return Ok(socket);
                        }
                        Err(e) => {
                            // The error names the address, and the address
                            // holds the ticket: said without it.
                            why = describe(&e);
                            dial.failed()
                        }
                    }
                }
                DialStep::Unauthorized => {
                    if self.session.is_none() {
                        return Err(Missed::NotAdmitted);
                    }
                    let Some(session) = self.session.as_ref().filter(|_| !retaken) else {
                        return Err(Missed::Refused(
                            "the seat token is no longer good for this table".into(),
                        ));
                    };
                    retaken = true;
                    self.chair = self
                        .lobby
                        .take_back(session, &self.chair.game_id)
                        .await
                        .map_err(|e| Missed::Refused(format!("{e:#}")))?;
                    let (fresh, first) = TicketDial::start();
                    dial = fresh;
                    first
                }
                DialStep::GiveUp => return Err(Missed::Unreachable(why)),
                DialStep::Wait => return Err(Missed::Unreachable("the dial stalled".into())),
            };
        }
    }
}

/// Opens a seat socket with Nagle's algorithm off (`TCP_NODELAY`), as the
/// client does: an answer goes out when it is written, not after the table
/// has acknowledged what this seat sent before, which Linux delays by up to
/// 40 ms.
async fn open(url: &str) -> Result<Socket, tokio_tungstenite::tungstenite::Error> {
    let (socket, _) = tokio_tungstenite::connect_async_with_config(url, None, true).await?;
    Ok(socket)
}

/// The socket address of a gateway named by its HTTP address.
///
/// # Errors
/// When the address is neither `http://` nor `https://`.
pub fn ws_base(http: &str) -> anyhow::Result<String> {
    let http = http.trim_end_matches('/');
    if let Some(rest) = http.strip_prefix("https://") {
        Ok(format!("wss://{rest}"))
    } else if let Some(rest) = http.strip_prefix("http://") {
        Ok(format!("ws://{rest}"))
    } else {
        Err(anyhow!(
            "a gateway is named by its http:// or https:// address, not «{http}»"
        ))
    }
}

/// A failed upgrade in words, never with its address.
fn describe(error: &tokio_tungstenite::tungstenite::Error) -> String {
    use tokio_tungstenite::tungstenite::Error;
    match error {
        Error::Http(response) => format!("the upgrade was answered {}", response.status()),
        Error::Io(e) => format!("the connection failed: {}", e.kind()),
        Error::Tls(_) => "the TLS handshake failed".into(),
        _ => "the socket did not open".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gateway_address_becomes_its_socket_address() {
        assert_eq!(
            ws_base("http://127.0.0.1:28766").unwrap(),
            "ws://127.0.0.1:28766"
        );
        assert_eq!(
            ws_base("https://play.example/").unwrap(),
            "wss://play.example"
        );
        assert!(ws_base("unix:/run/baylee.sock").is_err());
    }

    /// A seat socket is opened with Nagle's algorithm off.
    #[tokio::test]
    async fn a_seat_socket_sends_at_once() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("ws://{}/games/g/ws", listener.local_addr().unwrap());
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let _ = tokio_tungstenite::accept_async(stream).await;
        });
        let socket = open(&url).await.unwrap();
        let MaybeTlsStream::Plain(tcp) = socket.get_ref() else {
            panic!("a plain TCP socket");
        };
        assert!(tcp.nodelay().unwrap());
    }
}
