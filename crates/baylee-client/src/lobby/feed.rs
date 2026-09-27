//! The lobby's live listing: one socket, pushed at, instead of a question
//! asked twice a second.
//!
//! The list a player is looking at changes because *somebody else* did
//! something — sat down, said they were ready, closed a room. Polling for that
//! is a request per client per two seconds, every one of which answers "no
//! change", and it still shows the news two seconds late. `/lobby/ws` sends
//! the page this client is reading whenever anything in the lobby moves.
//!
//! What the socket is opened for is the *query*, not just the account: a
//! search and a page are part of the subscription, so typing in the search box
//! re-dials. That is why the URL is built from the same [`GameQuery`] the HTTP
//! path uses — a socket answering a different question than the Refresh button
//! would be the worst of both.
//!
//! Every dial first buys a ticket with the session (#294), so the session
//! never rides in the socket's address; see
//! [`baylee_client_core::wsticket`] for the sequence and its retries.

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;
use baylee_client_core::wsticket::{Socket as TicketSocket, Step, TicketDial};

/// How long to wait before dialling again after a socket failed.
///
/// A gateway that is down is down for longer than this, and the cost of being
/// wrong is one connection attempt — while a client that never re-dials shows
/// a stale lobby with no sign that it is stale.
const REDIAL_SECS: f32 = 4.0;

/// The push socket, and what it was opened for.
#[derive(Resource, Default)]
pub(super) struct Feed {
    /// The live socket, once there is one.
    ///
    /// Behind a `Mutex` only to be `Sync`, which a Bevy resource must be:
    /// `ewebsock`'s receiver is a plain channel. Nothing else contends for
    /// it — one system touches it, once a frame.
    link: Mutex<Option<Link>>,
    /// The account token and query the socket was opened for. A change here
    /// is what makes it re-dial.
    asked: Option<(String, GameQuery)>,
    /// The ticket request of the dial under way, if one is waiting.
    pending: Option<crate::net::PendingTicket>,
    /// Where the dial under way stands; `None` between dials.
    dial: Option<TicketDial>,
    /// The gateway epoch the dial began in, so a session refused on a
    /// gateway this lobby has since left signs nobody out.
    epoch: u64,
    /// Seconds left before another attempt, after one failed.
    cooldown: f32,
}

/// One live listing socket.
struct Link {
    /// Held only to keep the socket open — the lobby feed never sends.
    _sender: crate::net::SocketSender,
    /// Incoming frames, drained without blocking.
    receiver: ewebsock::WsReceiver,
    /// Whether a listing has arrived on it. Until one has, this socket has
    /// not replaced the polling it exists to make unnecessary.
    delivered: bool,
}

impl Feed {
    /// Whether the list is arriving by itself.
    pub(super) fn live(&self) -> bool {
        self.link
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .is_some_and(|l| l.delivered)
    }

    /// Closes the socket, whatever state it was in, and forgets any ticket
    /// still on its way.
    fn hang_up(&mut self) {
        self.set_link(None);
        self.asked = None;
        self.pending = None;
        self.dial = None;
    }

    fn set_link(&self, link: Option<Link>) {
        *self
            .link
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = link;
    }
}

/// Keeps the socket pointed at the page the player is reading, and posts what
/// arrives on it into the mailbox.
pub(super) fn feed(
    time: Res<Time>,
    state: Res<LobbyState>,
    mailbox: Res<Mailbox>,
    mut feed: ResMut<Feed>,
) {
    let feed = &mut *feed;
    // Only the table screen reads the list. The deck builder is a long visit
    // and the sign-in screen has no token, so a socket held open across
    // either is a subscription nobody is reading.
    let wanted = match (state.lobby.token(), state.lobby.screen()) {
        (Some(token), Screen::Table) => Some((token.to_string(), state.lobby.query())),
        _ => None,
    };
    let Some(wanted) = wanted else {
        feed.hang_up();
        return;
    };
    if feed.asked.as_ref() != Some(&wanted) {
        feed.hang_up();
        if feed.cooldown > 0.0 {
            feed.cooldown -= time.delta_secs();
            return;
        }
        let (dial, _) = TicketDial::start();
        feed.pending = Some(crate::net::request_ticket(
            &state.gateway,
            &wanted.0,
            TicketSocket::Lobby,
        ));
        feed.dial = Some(dial);
        feed.epoch = state.gateway_epoch;
        feed.asked = Some(wanted);
        return;
    }
    if let Some(answer) = feed
        .pending
        .as_ref()
        .and_then(crate::net::PendingTicket::take)
    {
        feed.pending = None;
        let step = feed
            .dial
            .as_mut()
            .map_or(Step::Wait, |d| d.answered(answer));
        perform(feed, &state, &mailbox, step);
        return;
    }
    let mut lost = false;
    let mut retry = None;
    {
        let mut held = feed
            .link
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(link) = held.as_mut() else {
            return;
        };
        while let Some(event) = link.receiver.try_recv() {
            match event {
                ewebsock::WsEvent::Message(ewebsock::WsMessage::Text(text)) => {
                    // A frame we cannot read is not worth a red line across
                    // the lobby: the list on screen is still the list, and
                    // the next push is one chair away.
                    if let Ok(listing) =
                        serde_json::from_str::<baylee_client_core::lobby::GameListing>(&text)
                    {
                        link.delivered = true;
                        post(&mailbox, LobbyEvent::Games(listing));
                    } else {
                        debug!("the lobby feed sent something unreadable");
                    }
                }
                ewebsock::WsEvent::Opened => {
                    if let Some(dial) = feed.dial.as_mut() {
                        dial.opened();
                    }
                }
                ewebsock::WsEvent::Error(_) | ewebsock::WsEvent::Closed => {
                    let step = feed.dial.as_mut().map_or(Step::GiveUp, TicketDial::failed);
                    if step == Step::FetchTicket {
                        retry = Some(step);
                    } else {
                        lost = true;
                    }
                    break;
                }
                ewebsock::WsEvent::Message(_) => {}
            }
        }
    }
    if let Some(step) = retry {
        perform(feed, &state, &mailbox, step);
    } else if lost {
        give_up(feed, &state, &mailbox);
    }
}

/// Does what the dial said to do next.
fn perform(feed: &mut Feed, state: &LobbyState, mailbox: &Mailbox, mut step: Step) {
    loop {
        match step {
            Step::Dial(ticket) => {
                let query = feed
                    .asked
                    .as_ref()
                    .map(|(_, q)| q.clone())
                    .unwrap_or_default();
                match open(&feed_url(&state.gateway, &ticket, &query)) {
                    Ok(link) => {
                        feed.set_link(Some(link));
                        return;
                    }
                    Err(reason) => {
                        debug!(reason, "could not open the lobby feed");
                        step = feed.dial.as_mut().map_or(Step::GiveUp, TicketDial::failed);
                    }
                }
            }
            Step::FetchTicket => {
                // Refused before it opened: the ticket is spent or stale, and
                // a fresh one is one request away. Nobody is told.
                feed.set_link(None);
                if let Some((token, _)) = feed.asked.as_ref() {
                    feed.pending = Some(crate::net::request_ticket(
                        &state.gateway,
                        token,
                        TicketSocket::Lobby,
                    ));
                }
                return;
            }
            // The gateway no longer knows the session: the player signs in
            // again, as after any other signed `401`.
            Step::Unauthorized => {
                let epoch = feed.epoch;
                feed.hang_up();
                feed.cooldown = REDIAL_SECS;
                if let Ok(mut box_) = mailbox.0.lock() {
                    box_.push(Reply::Remote(epoch, Box::new(Reply::Expired)));
                }
                return;
            }
            Step::GiveUp => {
                give_up(feed, state, mailbox);
                return;
            }
            Step::Wait => return,
        }
    }
}

/// The dial is over without a socket: wait, then try again, and say the
/// gateway is gone meanwhile.
fn give_up(feed: &mut Feed, state: &LobbyState, mailbox: &Mailbox) {
    feed.hang_up();
    feed.cooldown = REDIAL_SECS;
    // Said at once rather than when the next poll times out; the next
    // listing, from either path, says it is back. A socket that named no
    // gateway lost none.
    if state.gateway.contains("://") {
        post(mailbox, LobbyEvent::GatewayLost);
    }
}

/// Leaves an event where [`super::poll`] will find it next frame.
fn post(mailbox: &Mailbox, event: LobbyEvent) {
    if let Ok(mut box_) = mailbox.0.lock() {
        box_.push(Reply::Event(event));
    }
}

/// The feed's websocket URL for a ticket and the page it subscribes to. The
/// session is not in it (#294): only the ticket it bought.
pub(crate) fn feed_url(gateway: &str, ticket: &str, query: &GameQuery) -> String {
    format!(
        "{}{}",
        crate::net::ws_base(gateway),
        baylee_client_core::wsticket::lobby_socket_path(
            &super::http::escape(ticket),
            &super::http::params(query)
        )
    )
}

/// Opens the socket at `url`.
fn open(url: &str) -> Result<Link, String> {
    let (sender, receiver) = crate::transport::ws_connect(url, ewebsock::Options::default())?;
    Ok(Link {
        _sender: crate::net::wrap_sender(sender),
        receiver,
        delivered: false,
    })
}
