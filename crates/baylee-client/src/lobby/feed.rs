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

#[allow(clippy::wildcard_imports)] // the lobby's own vocabulary
use super::*;

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

    /// Closes the socket, whatever state it was in.
    fn hang_up(&mut self) {
        *self
            .link
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        self.asked = None;
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
        match dial(&state.gateway, &wanted.0, &wanted.1) {
            Some(link) => {
                *feed
                    .link
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(link);
                feed.asked = Some(wanted);
            }
            None => feed.cooldown = REDIAL_SECS,
        }
        return;
    }
    let mut held = feed
        .link
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let Some(link) = held.as_mut() else {
        return;
    };
    let mut lost = false;
    while let Some(event) = link.receiver.try_recv() {
        match event {
            ewebsock::WsEvent::Message(ewebsock::WsMessage::Text(text)) => {
                // A frame we cannot read is not worth a red line across the
                // lobby: the list on screen is still the list, and the next
                // push is one chair away.
                if let Ok(listing) =
                    serde_json::from_str::<baylee_client_core::lobby::GameListing>(&text)
                {
                    link.delivered = true;
                    post(&mailbox, LobbyEvent::Games(listing));
                } else {
                    debug!("the lobby feed sent something unreadable");
                }
            }
            ewebsock::WsEvent::Error(_) | ewebsock::WsEvent::Closed => lost = true,
            ewebsock::WsEvent::Opened | ewebsock::WsEvent::Message(_) => {}
        }
    }
    drop(held);
    if lost {
        feed.hang_up();
        feed.cooldown = REDIAL_SECS;
    }
}

/// Leaves an event where [`super::poll`] will find it next frame.
fn post(mailbox: &Mailbox, event: LobbyEvent) {
    if let Ok(mut box_) = mailbox.0.lock() {
        box_.push(Reply::Event(event));
    }
}

/// Opens the socket for one query.
fn dial(gateway: &str, token: &str, query: &GameQuery) -> Option<Link> {
    let base = crate::net::ws_base(gateway);
    let url = format!(
        "{base}/lobby/ws?token={}&{}",
        super::http::escape(token),
        super::http::params(query)
    );
    match ewebsock::connect(url, ewebsock::Options::default()) {
        Ok((sender, receiver)) => Some(Link {
            _sender: crate::net::wrap_sender(sender),
            receiver,
            delivered: false,
        }),
        Err(reason) => {
            debug!(reason, "could not open the lobby feed");
            None
        }
    }
}
