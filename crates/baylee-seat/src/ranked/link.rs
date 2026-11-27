//! An AI seat's way to its table: through Baylee Central's relay, as the
//! native client goes (`crates/baylee-client/src/lobby/ranked/native.rs`),
//! for a match Central made from the account's standing offer, which no
//! queue entry holds: the match is named by its id, and Central's `Work`
//! says what it is now ([`Account::play_ai_seat`]).
//!
//! Every dial asks Central's relay for a seat: the relay takes the
//! match-bound admission for the account (`POST /auth/ranked/login` at the
//! host), hands the host the frozen deck the first time (`POST
//! /ranked/{match}/play`, answered `202` until both seats registered, and
//! asked again with the same admission until it seats) and the game's id
//! every time after (`{"game": …}`, a resume), and trades the seat for a
//! one-use socket ticket. The socket is Central's (`/v1/ranked/socket`),
//! opened with that ticket as its first frame; the runner never dials the
//! host itself. A socket that drops is replaced the same way, and the core
//! asks the table for what it missed.
//!
//! Before the first seat, Central asks every participant to approve the
//! match's hosts and their terms: the link approves only hosts on the
//! operator's list (`--accept-host`) and declines any other, which releases
//! the match without a score.
//!
//! While the seat holds a game, the admission is renewed at the `renew_in`
//! the host gave ([`Account::renew_ai_seat`]), so a long game keeps its
//! session at the host. The link holds the runner session it started on;
//! when Central ended that one, it goes on with the account's newest.
//!
//! Only a single-host match is played (`Phase::Single`, every ranked game
//! today): its frames are the ordinary seat frames and its answers go on
//! the socket. A replicated match (two hosts, each frame a certified
//! projection, each answer a journal entry at Central, presence every five
//! seconds) is refused in one sentence: that is a second link, still to be
//! built.
//!
//! [`Account::play_ai_seat`]: baylee_account::Account::play_ai_seat
//! [`Account::renew_ai_seat`]: baylee_account::Account::renew_ai_seat

use super::{Aborting, Frozen, Player, Session};
use crate::link::{Reporter, Socket, Table};
use anyhow::{anyhow, bail};
use baylee_account::ranked::Assignment;
use baylee_account::ranked::replica::{Fence, Phase, SeatRequest, SeatResponse};
use baylee_account::{Error, RankedHost, RankedPlay};
use baylee_client_core::reconnect::Window;
use futures_util::SinkExt as _;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;
use zeroize::Zeroizing;

/// How a ranked link waits and gives up.
#[derive(Clone, Debug)]
pub struct LinkOptions {
    /// Between two asks of a seat that is not ready yet (the other seat has
    /// not registered, a consent is still open).
    pub poll: Duration,
    /// How long a seat may stay unready, or its socket fail to open, before
    /// the link gives up on the game.
    pub give_up: Duration,
    /// The longest the link waits between two renewals of the admission,
    /// whatever `renew_in` the host gave: `None` keeps the host's. A test
    /// shortens it to see a renewal within one game.
    pub renew_cap: Option<Duration>,
    /// The host origins whose terms the account accepts on its own when a
    /// match is assigned to them (`--accept-host`); a match on any other
    /// host is declined, which releases it without a score.
    pub accept_hosts: Vec<String>,
}

impl Default for LinkOptions {
    fn default() -> Self {
        Self {
            poll: Duration::from_secs(1),
            give_up: Duration::from_secs(180),
            renew_cap: None,
            accept_hosts: Vec::new(),
        }
    }
}

/// One AI seat's way to its table.
pub struct RankedLink {
    player: Player,
    session: Session,
    assignment: Assignment,
    deck: Arc<Frozen>,
    options: LinkOptions,
    /// The admission the relay took while the host answered `202`: asked
    /// again with the same one until the seat is granted.
    pending: Option<RankedHost>,
    window: Window,
    dials: u32,
    /// The last socket ticket, which no report may carry.
    ticket: Zeroizing<String>,
    renewal: Option<Aborting>,
    renewals: Arc<AtomicU32>,
}

/// What one ask for a seat came to.
enum Seat {
    /// A seat and its socket ticket.
    Granted(Box<RankedPlay>),
    /// Not yet: ask again.
    Wait,
    /// The match is over or no longer this account's.
    Gone,
    /// The match is replicated across two hosts: not played here yet.
    Replicated,
    /// A host the operator did not list: the match was declined.
    Declined,
}

/// What the blocking call came back with.
enum Asked {
    Played(Option<Box<RankedPlay>>),
    Gone,
    Replicated,
}

impl RankedLink {
    /// The link to `assignment`, a match of `player`'s, with `deck` frozen
    /// for it, on the account's newest session.
    #[must_use]
    pub fn new(
        player: Player,
        assignment: Assignment,
        deck: Arc<Frozen>,
        options: LinkOptions,
    ) -> Self {
        Self {
            session: player.session(),
            player,
            assignment,
            deck,
            options,
            pending: None,
            window: Window::Unknown,
            dials: 0,
            ticket: Zeroizing::new(String::new()),
            renewal: None,
            renewals: Arc::default(),
        }
    }

    /// The match as Central last confirmed it (with its game once seated).
    #[must_use]
    pub const fn assignment(&self) -> &Assignment {
        &self.assignment
    }

    /// Admissions renewed so far.
    #[must_use]
    pub fn renewals(&self) -> u32 {
        self.renewals.load(Ordering::Relaxed)
    }

    /// Goes on with the account's newest session when Central ended the
    /// one the link held (`Rejected`); the admission it took goes with it.
    fn on_rejected(&mut self, error: Error) {
        let newest = self.player.session();
        if error == Error::Rejected && !newest.same(&self.session) {
            tracing::info!("the match goes on with the account's newest runner session");
            self.session = newest;
            self.pending = None;
        }
    }

    async fn seat(&mut self) -> Result<Seat, Error> {
        let match_id = self.assignment.match_id;
        let deck = Arc::clone(&self.deck);
        let mut pending = self.pending.take();
        let (pending, asked) = self
            .session
            .call(move |account| {
                let SeatResponse::Status { status } =
                    account.replica(SeatRequest::Status { match_id })?
                else {
                    return Err(Error::Rejected);
                };
                match status.phase {
                    Phase::Single => {}
                    Phase::Finished | Phase::Void => return Ok((pending, Ok(Asked::Gone))),
                    _ => return Ok((pending, Ok(Asked::Replicated))),
                }
                let played =
                    account.play_ai_seat(match_id, Some((deck.digest, &deck.deck)), &mut pending);
                Ok((pending, played.map(|p| Asked::Played(p.map(Box::new)))))
            })
            .await?;
        self.pending = pending;
        match asked {
            Ok(Asked::Played(Some(play))) => Ok(Seat::Granted(play)),
            Ok(Asked::Played(None)) => Ok(Seat::Wait),
            Ok(Asked::Gone) => Ok(Seat::Gone),
            Ok(Asked::Replicated) => Ok(Seat::Replicated),
            Err(Error::TermsRequired) => Ok(if self.consent().await? {
                Seat::Wait
            } else {
                Seat::Declined
            }),
            Err(Error::Conflict) => {
                if self.gone().await {
                    Ok(Seat::Gone)
                } else {
                    Err(Error::Conflict)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// The match's hosts asked for the account's consent: given where every
    /// one of them is on the operator's list, else declined, which releases
    /// the match without a score. Whether it was given.
    async fn consent(&self) -> Result<bool, Error> {
        let match_id = self.assignment.match_id;
        let allowed = self.options.accept_hosts.clone();
        let accepted = self
            .session
            .call(move |account| {
                let documents = account.ranked_host_documents(match_id)?;
                if documents
                    .hosts
                    .iter()
                    .all(|h| allowed.contains(&h.offer.host.origin))
                {
                    documents.accept(account)?;
                    Ok(true)
                } else {
                    documents.decline(account)?;
                    Ok(false)
                }
            })
            .await?;
        if accepted {
            tracing::info!(origin = %self.assignment.origin, "accepted the host's terms for this match");
        } else {
            tracing::warn!(
                origin = %self.assignment.origin,
                "declined a match on a host the operator did not list (--accept-host)"
            );
        }
        Ok(accepted)
    }

    /// Whether the match is over or no longer this account's, as Central
    /// says; `false` where it cannot say.
    async fn gone(&self) -> bool {
        let match_id = self.assignment.match_id;
        let state = self
            .session
            .call(move |account| {
                match account.ai_assignment(match_id) {
                    Ok(_) => {}
                    Err(Error::Conflict) => return Ok(true),
                    Err(e) => return Err(e),
                }
                let SeatResponse::Status { status } =
                    account.replica(SeatRequest::Status { match_id })?
                else {
                    return Err(Error::Rejected);
                };
                Ok(matches!(status.phase, Phase::Finished | Phase::Void))
            })
            .await;
        state.unwrap_or_else(|e| {
            tracing::info!(error = %e, "Central could not say whether the match is on");
            false
        })
    }

    /// Opens the relay's socket with the ticket `play` carries, and keeps
    /// the admission renewed from then on.
    async fn dial(&mut self, play: RankedPlay) -> anyhow::Result<Socket> {
        let url = socket_url(&play.relay_origin)?;
        let mut socket = crate::link::open_socket(&url)
            .await
            .map_err(|why| anyhow!("the relay's socket did not open: {why}"))?;
        socket
            .send(Message::Binary(
                play.seat.seat_token.as_bytes().to_vec().into(),
            ))
            .await
            .map_err(|_| anyhow!("the relay's socket closed before its ticket"))?;
        self.ticket = Zeroizing::new(play.seat.seat_token.as_str().to_owned());
        self.assignment = play.assignment.clone();
        self.renewal = Some(self.renew(play.assignment, play.fence, play.renew_at.into()));
        Ok(socket)
    }

    /// Renews the admission at each `renew_in` until dropped, on the
    /// session that took it.
    fn renew(&self, assignment: Assignment, fence: Fence, first: Instant) -> Aborting {
        let (session, cap) = (self.session.clone(), self.options.renew_cap);
        let renewals = Arc::clone(&self.renewals);
        let capped = move |at: Instant| cap.map_or(at, |cap| at.min(Instant::now() + cap));
        Aborting(tokio::spawn(async move {
            let mut at = capped(first);
            loop {
                tokio::time::sleep_until(at).await;
                let game = assignment.clone();
                match session
                    .call(move |account| account.renew_ai_seat(&game, fence))
                    .await
                {
                    Ok(next) => {
                        renewals.fetch_add(1, Ordering::Relaxed);
                        tracing::info!("the ranked admission was renewed");
                        at = capped(next.into());
                    }
                    Err(e) => {
                        tracing::info!(error = %e, "the ranked admission was not renewed");
                        at = Instant::now() + Duration::from_secs(15);
                    }
                }
            }
        }))
    }
}

impl Table for RankedLink {
    fn hold_for(&mut self, window: Window) {
        self.window = window;
    }

    async fn connect(&mut self) -> anyhow::Result<Option<Socket>> {
        let since = Instant::now();
        let mut wait = self.options.poll;
        loop {
            let failed = match self.seat().await {
                Ok(Seat::Granted(play)) => match self.dial(*play).await {
                    Ok(socket) => {
                        self.dials += 1;
                        return Ok(Some(socket));
                    }
                    Err(e) => format!("{e:#}"),
                },
                Ok(Seat::Gone) => return Ok(None),
                Ok(Seat::Wait) => String::new(),
                Ok(Seat::Replicated) => bail!(
                    "this match is replicated across two hosts, which the ranked runner \
                     does not play yet"
                ),
                Ok(Seat::Declined) => {
                    bail!("the match's host is not one the operator accepts (--accept-host)")
                }
                Err(e) => {
                    self.on_rejected(e);
                    e.to_string()
                }
            };
            if since.elapsed() >= self.options.give_up {
                bail!(
                    "no seat at the match's host after {} s{}",
                    self.options.give_up.as_secs(),
                    if failed.is_empty() {
                        String::new()
                    } else {
                        format!(": {failed}")
                    }
                );
            }
            if failed.is_empty() {
                wait = self.options.poll;
            } else {
                tracing::info!(why = %failed, wait_s = wait.as_secs_f32(), "no ranked seat yet");
                wait = (wait * 2).min(Duration::from_secs(15));
            }
            tokio::time::sleep(wait).await;
        }
    }

    async fn room_closed(&mut self) -> bool {
        self.gone().await
    }

    fn selected_deck(
        &self,
    ) -> impl Future<Output = anyhow::Result<Option<crate::mind::DeckList>>> + Send {
        std::future::ready(Ok(None))
    }

    fn reporter(&self) -> Option<Reporter<'_>> {
        None
    }

    fn seat_secret(&self) -> &str {
        &self.ticket
    }

    fn dials(&self) -> u32 {
        self.dials
    }
}

/// The relay's socket at Central's `origin`: `wss://` for `https://`, and
/// `ws://` only for a loopback development Central, as the client allows
/// (`crates/baylee-client/src/net/replica.rs` `socket_url`).
///
/// # Errors
/// Any other origin, or one with credentials, a path, a query or a fragment.
pub fn socket_url(origin: &str) -> anyhow::Result<String> {
    use url::{Host, Url};
    let invalid = || anyhow!("Central's origin is not one a ranked socket may open");
    let mut url = Url::parse(origin).map_err(|_| invalid())?;
    let host = url.host().ok_or_else(invalid)?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    let scheme = match (url.scheme(), host) {
        ("https", _) => "wss",
        ("http", Host::Ipv4(ip)) if ip.is_loopback() => "ws",
        ("http", Host::Ipv6(ip)) if ip.is_loopback() => "ws",
        _ => return Err(invalid()),
    };
    url.set_scheme(scheme).map_err(|()| invalid())?;
    url.set_path("/v1/ranked/socket");
    Ok(url.into())
}
