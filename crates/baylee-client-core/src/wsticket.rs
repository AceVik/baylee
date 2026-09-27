//! Opening a socket with a ticket (#294).
//!
//! A browser's `WebSocket` cannot carry a header, so the client used to put
//! its secret in the socket's address — the session on the lobby feed, the
//! seat token on a seat — where a proxy's access log would see it. Now every
//! dial is two steps: ask the gateway for a ticket over an ordinary request
//! that carries the bearer in `Authorization` (`POST /ws-ticket`), then open
//! the socket with `?ticket=`. A ticket opens one socket, once, within
//! seconds, so it is harmless wherever the address ends up.
//!
//! This module knows no transport, which is what makes the sequence
//! testable: the shell asks [`TicketDial`] what to do next, performs it, and
//! reports back. The rules it enforces:
//!
//! - a fresh ticket before **every** dial, the first and every reconnect;
//! - an upgrade that fails before the socket opened is retried at once with a
//!   fresh ticket, up to [`STALE_TICKET_RETRIES`] times, without the player
//!   seeing anything: the ticket may simply have expired or been spent. A
//!   browser cannot read why an upgrade failed, so every such failure counts,
//!   which is the owner's rule for it (27.09.2026). After that the dial gives
//!   up and the ordinary "cannot reach the gateway" back-off takes over;
//! - a `401` on the ticket request itself means the credential is gone — for
//!   the lobby, the session: the player signs in again.

use serde::Deserialize;

/// How many times a dial whose upgrade failed fetches a fresh ticket and
/// tries again before it gives up.
pub const STALE_TICKET_RETRIES: u8 = 2;

/// Which socket a ticket is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Socket<'a> {
    /// The lobby feed (`/lobby/ws`), bought with the session.
    Lobby,
    /// One seat's socket (`/games/{game_id}/ws`), bought with its seat token.
    Seat {
        /// The game.
        game_id: &'a str,
    },
}

impl Socket<'_> {
    /// The body of the `POST` to [`baylee_protocol::WS_TICKET_PATH`].
    #[must_use]
    pub fn request_body(self) -> String {
        match self {
            Self::Lobby => serde_json::json!({ "socket": "lobby" }),
            Self::Seat { game_id } => serde_json::json!({ "socket": "seat", "game": game_id }),
        }
        .to_string()
    }
}

/// What the gateway answered a ticket request, read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TicketAnswer {
    /// A ticket, good for one upgrade.
    Ticket(String),
    /// `401`: the bearer is not (or no longer) good for this socket.
    Unauthorized,
    /// Anything else: another status, an unreadable body, or no answer at
    /// all. Said in words for the log, never with the bearer in them.
    Failed(String),
}

impl TicketAnswer {
    /// Reads a response to the ticket request.
    #[must_use]
    pub fn read(status: u16, body: &[u8]) -> Self {
        #[derive(Deserialize)]
        struct Body {
            ticket: String,
        }
        match status {
            200 => match serde_json::from_slice::<Body>(body) {
                Ok(Body { ticket }) if !ticket.is_empty() => Self::Ticket(ticket),
                _ => Self::Failed("the gateway's ticket was unreadable".to_string()),
            },
            401 => Self::Unauthorized,
            other => Self::Failed(format!("the gateway answered {other} for a socket ticket")),
        }
    }
}

/// What the shell should do next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Ask for a ticket.
    FetchTicket,
    /// Open the socket with this ticket.
    Dial(String),
    /// Nothing: wait for the next report.
    Wait,
    /// The credential is gone (a `401` on the ticket request).
    Unauthorized,
    /// This dial is over; the caller's own back-off decides when to start
    /// another.
    GiveUp,
}

/// Where one dial stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Fetching,
    Dialling,
    Open,
    Over,
}

/// One dial of one socket, from the ticket request to the open socket.
///
/// A new one per dial: [`TicketDial::start`] is the first ticket request of
/// a connect or a reconnect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TicketDial {
    phase: Phase,
    retries_left: u8,
}

impl TicketDial {
    /// A dial that begins by asking for a ticket.
    #[must_use]
    pub const fn start() -> (Self, Step) {
        (
            Self {
                phase: Phase::Fetching,
                retries_left: STALE_TICKET_RETRIES,
            },
            Step::FetchTicket,
        )
    }

    /// The ticket request was answered.
    pub fn answered(&mut self, answer: TicketAnswer) -> Step {
        if self.phase != Phase::Fetching {
            return Step::Wait;
        }
        match answer {
            TicketAnswer::Ticket(ticket) => {
                self.phase = Phase::Dialling;
                Step::Dial(ticket)
            }
            TicketAnswer::Unauthorized => {
                self.phase = Phase::Over;
                Step::Unauthorized
            }
            TicketAnswer::Failed(_) => {
                self.phase = Phase::Over;
                Step::GiveUp
            }
        }
    }

    /// The socket opened: the ticket did its work.
    pub fn opened(&mut self) {
        if self.phase == Phase::Dialling {
            self.phase = Phase::Open;
        }
    }

    /// The socket failed or closed. Before it opened, that is most likely a
    /// ticket that expired or was already spent, so a fresh one is fetched
    /// while retries last; after it opened, or once they are spent, the dial
    /// is over.
    pub fn failed(&mut self) -> Step {
        match self.phase {
            Phase::Dialling if self.retries_left > 0 => {
                self.retries_left -= 1;
                self.phase = Phase::Fetching;
                Step::FetchTicket
            }
            Phase::Fetching | Phase::Over => Step::Wait,
            Phase::Dialling | Phase::Open => {
                self.phase = Phase::Over;
                Step::GiveUp
            }
        }
    }

    /// Whether this dial is still under way: a ticket asked for, or a socket
    /// being opened, and neither given up nor open.
    #[must_use]
    pub fn in_flight(&self) -> bool {
        matches!(self.phase, Phase::Fetching | Phase::Dialling)
    }

    /// Whether the socket opened.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.phase == Phase::Open
    }
}

/// The lobby feed's path for a ticket and the search it subscribes to
/// (`q=…&offset=…`, already escaped, possibly empty).
#[must_use]
pub fn lobby_socket_path(ticket: &str, search: &str) -> String {
    if search.is_empty() {
        format!("/lobby/ws?ticket={ticket}")
    } else {
        format!("/lobby/ws?ticket={ticket}&{search}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticket(t: &str) -> TicketAnswer {
        TicketAnswer::Ticket(t.to_string())
    }

    #[test]
    fn a_dial_asks_for_a_ticket_and_then_opens_with_it() {
        let (mut dial, step) = TicketDial::start();
        assert_eq!(step, Step::FetchTicket);
        assert!(dial.in_flight());
        assert_eq!(dial.answered(ticket("t1")), Step::Dial("t1".into()));
        assert!(dial.in_flight());
        dial.opened();
        assert!(dial.is_open());
        assert!(!dial.in_flight());
    }

    /// The owner's rule: a failed upgrade is a stale ticket until proven
    /// otherwise, so the client fetches a fresh one and tries again — twice,
    /// silently — and succeeds without the player having done anything.
    #[test]
    fn a_stale_ticket_is_replaced_and_the_dial_succeeds() {
        let (mut dial, _) = TicketDial::start();
        assert_eq!(dial.answered(ticket("stale")), Step::Dial("stale".into()));
        assert_eq!(
            dial.failed(),
            Step::FetchTicket,
            "first refusal: a fresh ticket"
        );
        assert!(dial.in_flight(), "and nobody is told the link is down");
        assert_eq!(dial.answered(ticket("stale2")), Step::Dial("stale2".into()));
        assert_eq!(
            dial.failed(),
            Step::FetchTicket,
            "second refusal: once more"
        );
        assert_eq!(dial.answered(ticket("fresh")), Step::Dial("fresh".into()));
        dial.opened();
        assert!(dial.is_open());
    }

    #[test]
    fn the_retries_run_out_and_the_ordinary_back_off_takes_over() {
        let (mut dial, _) = TicketDial::start();
        for n in 0..STALE_TICKET_RETRIES {
            assert_eq!(dial.answered(ticket("t")), Step::Dial("t".into()));
            assert_eq!(dial.failed(), Step::FetchTicket, "retry {n}");
        }
        assert_eq!(dial.answered(ticket("t")), Step::Dial("t".into()));
        assert_eq!(dial.failed(), Step::GiveUp);
        assert!(!dial.in_flight());
        assert_eq!(dial.failed(), Step::Wait, "a dial gives up once");
    }

    /// A socket that opened and later dropped is not a stale ticket: it is
    /// a lost connection, and the reconnect schedule's to handle.
    #[test]
    fn a_drop_after_opening_is_not_retried_here() {
        let (mut dial, _) = TicketDial::start();
        dial.answered(ticket("t"));
        dial.opened();
        assert_eq!(dial.failed(), Step::GiveUp);
    }

    #[test]
    fn a_refused_ticket_request_ends_the_dial() {
        let (mut dial, _) = TicketDial::start();
        assert_eq!(
            dial.answered(TicketAnswer::Unauthorized),
            Step::Unauthorized
        );
        assert!(!dial.in_flight());
        let (mut dial, _) = TicketDial::start();
        assert_eq!(
            dial.answered(TicketAnswer::Failed("503".into())),
            Step::GiveUp
        );
        assert_eq!(
            dial.answered(ticket("late")),
            Step::Wait,
            "a late ticket is dropped"
        );
    }

    #[test]
    fn the_gateway_s_answer_is_read() {
        assert_eq!(
            TicketAnswer::read(200, br#"{"ticket":"abc","expires_in":45}"#),
            ticket("abc")
        );
        assert_eq!(TicketAnswer::read(401, b"{}"), TicketAnswer::Unauthorized);
        assert!(matches!(
            TicketAnswer::read(200, b"nope"),
            TicketAnswer::Failed(_)
        ));
        assert!(matches!(
            TicketAnswer::read(200, br#"{"ticket":""}"#),
            TicketAnswer::Failed(_)
        ));
        assert!(matches!(
            TicketAnswer::read(404, b"{}"),
            TicketAnswer::Failed(_)
        ));
    }

    #[test]
    fn the_request_names_the_socket_and_nothing_secret() {
        assert_eq!(Socket::Lobby.request_body(), r#"{"socket":"lobby"}"#);
        let seat = Socket::Seat { game_id: "g\"1" }.request_body();
        let parsed: serde_json::Value = serde_json::from_str(&seat).expect("json");
        assert_eq!(parsed["socket"], "seat");
        assert_eq!(parsed["game"], "g\"1", "escaped, not spliced");
    }

    #[test]
    fn the_lobby_path_carries_a_ticket_and_the_search() {
        assert_eq!(lobby_socket_path("t", ""), "/lobby/ws?ticket=t");
        assert_eq!(
            lobby_socket_path("t", "q=x&offset=0"),
            "/lobby/ws?ticket=t&q=x&offset=0"
        );
        assert!(!lobby_socket_path("t", "q=x").contains("token="));
    }
}
