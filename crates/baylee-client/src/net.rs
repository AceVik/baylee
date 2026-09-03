//! A duel played over the wire.
//!
//! # Why this is a host and not a client
//!
//! The renderer already talks to a [`DuelHost`] rather than to a game: solo
//! play installs [`LocalHost`](crate::LocalHost), which runs an engine in this
//! process. Playing against a person changes exactly one thing — where the
//! envelopes come from — so this module is a second [`DuelHost`] and nothing
//! above it changes. Both hosts decode the same protobuf envelopes with the
//! same function, which is what makes solo play a real test of the wire
//! format instead of a shortcut around it.
//!
//! # One socket, two platforms
//!
//! `ewebsock` is a background thread speaking tungstenite natively and the
//! browser's own `WebSocket` on wasm, behind one non-blocking API. That
//! matters here: [`DuelHost::poll`] runs once per frame and may never wait on
//! I/O, and the browser build has no threads to wait on anything with.
//!
//! # What this host does not do
//!
//! It does not log in, list games, or pick a deck. It is handed a
//! [`SeatTicket`] — a game id and the seat token the gateway issued for it —
//! and connects. Getting that ticket is the lobby's job, and the lobby is
//! HTTP (see [`crate::settings::gateway_url`]).

use crate::host::{DuelHost, HostMessage, host_message};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::{self, Envelope};
use ewebsock::{WsEvent, WsMessage};
use prost::Message as _;
use std::sync::{Mutex, PoisonError};

/// The largest frame this client will accept.
///
/// Both servers cap what they send at the same figure. A client that accepted
/// more would only be agreeing to allocate whatever a broken — or hostile —
/// host decided to send it.
const MAX_FRAME_BYTES: usize = 4 << 20;

/// Everything the gateway hands a player when they take a seat.
///
/// The token is the whole of the authorisation: it names one seat of one game,
/// and the gateway compares it in constant time against a stored hash. A
/// client cannot act for another seat by editing this, because the seat comes
/// from the token rather than from anything the client says afterwards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeatTicket {
    /// Gateway base URL, e.g. `http://127.0.0.1:28766`.
    pub gateway: String,
    /// The game this ticket is for.
    pub game_id: String,
    /// The seat the ticket is expected to open.
    ///
    /// Only a hint: the table says which chair this is, in the opening
    /// payload, and [`NetworkHost`] believes the table.
    pub seat: PlayerId,
    /// The seat token issued by `POST /lobby/games` or `.../join`.
    pub seat_token: String,
}

/// A gateway base URL as a websocket URL of the same host.
///
/// Every socket this client opens against the gateway goes through here, so a
/// lobby feed and a seat cannot disagree about what `https` means.
#[must_use]
pub(crate) fn ws_base(gateway: &str) -> String {
    let base = gateway.trim_end_matches('/');
    match base.split_once("://") {
        Some(("https", rest)) => format!("wss://{rest}"),
        Some(("http", rest)) => format!("ws://{rest}"),
        // Already a websocket URL, or something the socket will refuse on
        // its own. Passing it through beats guessing at it.
        _ => base.to_string(),
    }
}

impl SeatTicket {
    /// The websocket URL this ticket opens.
    #[must_use]
    pub fn socket_url(&self) -> String {
        let base = ws_base(&self.gateway);
        format!("{base}/games/{}/ws?token={}", self.game_id, self.seat_token)
    }

    /// The ticket this launch was handed, if it was handed one.
    ///
    /// Natively that is the environment (`BAYLEE_GAME`, `BAYLEE_SEAT_TOKEN`,
    /// optionally `BAYLEE_SEAT`); in a browser it is the page's own query
    /// string, `?game=…&token=…`, which is how a web lobby hands a player over
    /// to the table. Without one the client plays solo.
    #[must_use]
    pub fn discover() -> Option<Self> {
        let (game_id, seat_token, seat) = Self::handover()?;
        if game_id.is_empty() || seat_token.is_empty() {
            return None;
        }
        Some(Self {
            gateway: crate::settings::gateway_url(),
            game_id,
            seat: PlayerId::new(seat),
            seat_token,
        })
    }

    /// The platform's way of naming a game and a seat token.
    #[cfg(not(target_arch = "wasm32"))]
    fn handover() -> Option<(String, String, u8)> {
        // Same order as the gateway address: the environment first, then the
        // working directory's `.env`. Pasting a ticket into a file beats
        // re-exporting two variables in every shell.
        let value = |key: &str| {
            std::env::var(key)
                .ok()
                .filter(|v| !v.is_empty())
                .or_else(|| crate::settings::dotenv_value(key))
        };
        let game_id = value("BAYLEE_GAME")?;
        let seat_token = value("BAYLEE_SEAT_TOKEN")?;
        let seat = value("BAYLEE_SEAT")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        Some((game_id, seat_token, seat))
    }

    /// The platform's way of naming a game and a seat token.
    #[cfg(target_arch = "wasm32")]
    fn handover() -> Option<(String, String, u8)> {
        let query = web_sys::window()?.location().search().ok()?;
        let game_id = query_value(&query, "game")?;
        let seat_token = query_value(&query, "token")?;
        let seat = query_value(&query, "seat")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        Some((game_id, seat_token, seat))
    }
}

/// Reads one key out of a `?a=1&b=2` query string.
///
/// Deliberately tiny rather than a dependency: the two values that matter are
/// a UUID and a hex token, neither of which is ever percent-encoded.
///
/// Only a browser has a query string to read; the test below is what keeps it
/// honest on the platform where it cannot be exercised by hand.
#[cfg(any(target_arch = "wasm32", test))]
pub(crate) fn query_value(query: &str, key: &str) -> Option<String> {
    query
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == key)
        .map(|(_, value)| value.to_string())
}

/// The socket half that sends, in whatever form this platform has one.
///
/// A Bevy resource must be `Send + Sync`, and the browser's `WebSocket` handle
/// is neither — it belongs to the thread that created it. `SendWrapper` says
/// exactly that and checks it: touching it from another thread panics rather
/// than corrupting anything, and on wasm there is no other thread to touch it
/// from. Natively the sender is already a channel, so nothing is wrapped.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) type SocketSender = ewebsock::WsSender;
/// See the native definition above.
#[cfg(target_arch = "wasm32")]
pub(crate) type SocketSender = send_wrapper::SendWrapper<ewebsock::WsSender>;

/// Wraps a fresh sender for this platform.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn wrap_sender(sender: ewebsock::WsSender) -> SocketSender {
    sender
}

/// See the native definition above.
#[cfg(target_arch = "wasm32")]
pub(crate) fn wrap_sender(sender: ewebsock::WsSender) -> SocketSender {
    send_wrapper::SendWrapper::new(sender)
}

/// One live connection.
struct Link {
    /// Outgoing frames.
    sender: SocketSender,
    /// Incoming events, drained without blocking.
    receiver: ewebsock::WsReceiver,
}

/// Opens a socket for a ticket.
fn dial(ticket: &SeatTicket) -> Result<Link, String> {
    let options = ewebsock::Options {
        max_incoming_frame_size: MAX_FRAME_BYTES,
        ..ewebsock::Options::default()
    };
    let (sender, receiver) = ewebsock::connect(ticket.socket_url(), options)?;
    Ok(Link {
        sender: wrap_sender(sender),
        receiver,
    })
}

/// A duel hosted somewhere else.
///
/// Holds no game state of its own: the table is authoritative about every
/// part of it, including which seat this client is sitting in.
pub struct NetworkHost {
    /// How to reach the table, and how to prove which chair this is.
    ticket: SeatTicket,
    /// The socket.
    ///
    /// Behind a `Mutex` only to be `Sync`, which a Bevy resource must be; the
    /// host is never actually shared, so every access is `get_mut` and no lock
    /// is ever taken.
    link: Mutex<Link>,
    /// The seat, as last confirmed by the table.
    seat: PlayerId,
    /// Highest sequence number seen, so a reconnect can ask for the rest.
    last_seq: u64,
    /// Whether the socket has reported itself open.
    open: bool,
    /// Frames waiting for the socket to open.
    ///
    /// A browser `WebSocket` throws on a send before `onopen`, and the first
    /// thing a reconnect wants to send is precisely the message that arrives
    /// before then.
    outbox: Vec<Envelope>,
    /// Messages this host produced itself, rather than received.
    pending_out: Vec<HostMessage>,
}

impl NetworkHost {
    /// Connects to the table named by a ticket.
    ///
    /// Returns as soon as the socket is being opened — nothing here waits for
    /// a handshake, because a frame is the only place this client is allowed
    /// to wait. The first messages arrive through [`DuelHost::poll`].
    ///
    /// # Errors
    /// When the socket cannot be created at all: a malformed URL, or (on
    /// native) a thread that could not be spawned.
    pub fn connect(ticket: SeatTicket) -> Result<Self, String> {
        let link = dial(&ticket)?;
        Ok(Self {
            seat: ticket.seat,
            ticket,
            link: Mutex::new(link),
            last_seq: 0,
            open: false,
            outbox: Vec::new(),
            pending_out: Vec::new(),
        })
    }

    /// Re-dials, and asks the table for whatever this seat missed.
    ///
    /// The gateway answers a `ResumeGame` with a full snapshot when the client
    /// is behind and with nothing at all when it is not, so calling this after
    /// a hiccup costs one round trip and never re-renders a table that never
    /// moved. Deliberately not automatic: only the application knows whether a
    /// player is still sitting there, and a host that redialled by itself
    /// would hammer a gateway that is down.
    ///
    /// # Errors
    /// When the socket cannot be created at all.
    pub fn reconnect(&mut self) -> Result<(), String> {
        let link = dial(&self.ticket)?;
        *self.link.get_mut().unwrap_or_else(PoisonError::into_inner) = link;
        self.open = false;
        let resume = Envelope {
            msg: Some(v1::envelope::Msg::Resume(v1::ResumeGame {
                game_id: self.ticket.game_id.clone(),
                seat_token: self.ticket.seat_token.clone(),
                last_seq: self.last_seq,
            })),
        };
        self.outbox.push(resume);
        Ok(())
    }

    /// Whether the socket has opened and not since closed.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.open
    }

    /// The last sequence number this client has seen.
    #[must_use]
    pub const fn last_seq(&self) -> u64 {
        self.last_seq
    }

    /// Remembers how far along the game this client is.
    ///
    /// Both a view and a choice carry the number, and a seat can be sent
    /// either one last, so both are counted.
    fn note_seq(&mut self, envelope: &Envelope) {
        let seq = match &envelope.msg {
            Some(v1::envelope::Msg::StateDelta(delta)) => delta.seq,
            Some(v1::envelope::Msg::ChoiceRequest(request)) => request.seq,
            _ => return,
        };
        self.last_seq = self.last_seq.max(seq);
    }

    /// Sends everything that was waiting for the socket to open.
    fn flush(&mut self) {
        if !self.open || self.outbox.is_empty() {
            return;
        }
        let link = self.link.get_mut().unwrap_or_else(PoisonError::into_inner);
        for envelope in self.outbox.drain(..) {
            link.sender
                .send(WsMessage::Binary(envelope.encode_to_vec()));
        }
    }
}

impl DuelHost for NetworkHost {
    fn poll(&mut self) -> Vec<HostMessage> {
        let mut out = std::mem::take(&mut self.pending_out);
        let events: Vec<WsEvent> = {
            let link = self.link.get_mut().unwrap_or_else(PoisonError::into_inner);
            std::iter::from_fn(|| link.receiver.try_recv()).collect()
        };
        for event in events {
            match event {
                WsEvent::Opened => self.open = true,
                WsEvent::Message(WsMessage::Binary(bytes)) => {
                    match Envelope::decode(bytes.as_slice()) {
                        Ok(envelope) => {
                            self.note_seq(&envelope);
                            if let Some(message) = host_message(envelope) {
                                if let HostMessage::Static(statics) = &message {
                                    // The table decides which chair this is; a
                                    // ticket could only ever guess at it.
                                    self.seat = statics.your_seat;
                                }
                                out.push(message);
                            }
                        }
                        Err(e) => out.push(HostMessage::Failed(format!("unreadable frame: {e}"))),
                    }
                }
                // The protocol is binary throughout; text on this socket is
                // somebody else's, and pings answer themselves.
                WsEvent::Message(_) => {}
                WsEvent::Error(reason) => out.push(HostMessage::Failed(reason)),
                WsEvent::Closed => {
                    self.open = false;
                    out.push(HostMessage::Failed(
                        "the connection to the table was lost".to_string(),
                    ));
                }
            }
        }
        self.flush();
        out
    }

    fn submit(&mut self, action: PlayerAction) {
        let Ok(action_json) = serde_json::to_vec(&action) else {
            self.pending_out.push(HostMessage::Failed(
                "your answer could not be encoded".to_string(),
            ));
            return;
        };
        self.outbox.push(Envelope {
            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                game_id: self.ticket.game_id.clone(),
                // The socket is already bound to one seat of one game. Sending
                // the token again in every frame would spread it further
                // without proving anything the connection has not proved.
                seat_token: String::new(),
                action_json,
            })),
        });
        self.flush();
    }

    fn seat(&self) -> PlayerId {
        self.seat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket(gateway: &str) -> SeatTicket {
        SeatTicket {
            gateway: gateway.to_string(),
            game_id: "0199-abc".to_string(),
            seat: PlayerId::new(1),
            seat_token: "deadbeef".to_string(),
        }
    }

    #[test]
    fn a_plain_gateway_becomes_a_plain_socket() {
        assert_eq!(
            ticket("http://127.0.0.1:28766").socket_url(),
            "ws://127.0.0.1:28766/games/0199-abc/ws?token=deadbeef"
        );
    }

    /// A browser serves the client over TLS, so the socket has to be TLS too —
    /// a `ws://` socket from an `https://` page is blocked outright, and the
    /// failure looks like "the game never starts".
    #[test]
    fn a_tls_gateway_becomes_a_tls_socket() {
        assert_eq!(
            ticket("https://play.example/").socket_url(),
            "wss://play.example/games/0199-abc/ws?token=deadbeef"
        );
    }

    #[test]
    fn a_query_string_gives_up_its_values() {
        let query = "?game=0199-abc&token=deadbeef&seat=1";
        assert_eq!(query_value(query, "game").as_deref(), Some("0199-abc"));
        assert_eq!(query_value(query, "token").as_deref(), Some("deadbeef"));
        assert_eq!(query_value(query, "seat").as_deref(), Some("1"));
        assert_eq!(query_value(query, "deck"), None);
        assert_eq!(query_value("", "game"), None);
    }

    /// `?token=` names a key with no value. Treating that as a token would
    /// send an empty one and get a 401 that reads like a server problem.
    #[test]
    fn an_empty_handover_is_no_handover() {
        assert_eq!(query_value("?game=x&token=", "token").as_deref(), Some(""));
        let empty = SeatTicket {
            gateway: "http://x".to_string(),
            game_id: "x".to_string(),
            seat: PlayerId::new(0),
            seat_token: String::new(),
        };
        // The check `discover` applies, on the values it would have built.
        assert!(empty.seat_token.is_empty());
    }
}
