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

use crate::lobby::{Chair, Lobby, Session};
use anyhow::{anyhow, bail};
use baylee_client_core::reconnect::Retry;
use baylee_client_core::wsticket::{Step as DialStep, TicketDial};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

/// An open seat socket.
pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Why one dial did not open.
#[derive(Debug)]
enum Missed {
    /// The table does not take this seat's token, and it could not be
    /// traded for one it takes.
    Refused(String),
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
        }
    }

    /// The chair this link opens.
    #[must_use]
    pub const fn chair(&self) -> &Chair {
        &self.chair
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
    /// When the seat's token is refused and cannot be replaced, or after
    /// [`Retry::GIVE_UP`] dials that reached nothing.
    pub async fn connect(&mut self) -> anyhow::Result<Option<Socket>> {
        let mut retry = Retry::new();
        loop {
            match self.dial().await {
                Ok(socket) => {
                    self.dials += 1;
                    return Ok(Some(socket));
                }
                Err(_) if self.room_closed().await => return Ok(None),
                Err(Missed::Refused(why)) => bail!("the table refused this seat: {why}"),
                Err(Missed::Unreachable(why)) => {
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
    /// `false` when that cannot be told (no session to ask with, a lobby
    /// that did not answer): a seat does not leave a table on a guess.
    pub async fn room_closed(&self) -> bool {
        let Some(session) = self.session.as_ref() else {
            return false;
        };
        match self.lobby.room(session, &self.chair.game_id).await {
            Ok(room) => room.is_none_or(|room| !room.playing()),
            Err(e) => {
                tracing::info!(
                    error = %format!("{e:#}"),
                    "the lobby could not say whether the room is open"
                );
                false
            }
        }
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
                    match tokio_tungstenite::connect_async(format!("{base}{path}")).await {
                        Ok((socket, _)) => {
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
}
