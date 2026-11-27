//! Where an account's matches come from ([`Assignments`]).
//!
//! Today that is the queue a person uses ([`Queue`]): join with the frozen
//! deck, wait, take the one assignment Central makes for the entry. Central's
//! design for AI accounts (baylee-central `docs/ai-players.md` §4) replaces
//! it: a flagged account may not join; it keeps standing deck offers
//! (`Ai::Offer`) and reads its live matches, several at once (`Ai::Work`).
//! That source is one more implementation of [`Assignments`]; the runner and
//! [`super::play_match`] stay as they are.

use super::{Frozen, Match, Player};
use anyhow::{anyhow, bail};
use baylee_account::Error;
use baylee_account::ranked::replica::{SeatRequest, SeatResponse};
use baylee_account::ranked::{Format, Id, QueueState, Request, Response};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use uuid::Uuid;

/// A source of the account's matches.
pub trait Assignments: Send {
    /// The account's live matches now, each with the deck it was made
    /// with; asked again and again, so a match already being played may be
    /// named again (the runner skips it). Asked only while the runner wants
    /// another match: a source that must ask for one (a search, an offer)
    /// asks here.
    fn live(&mut self) -> impl Future<Output = anyhow::Result<Vec<Match>>> + Send;

    /// The runner stops: withdraw what still waits for an opponent (a
    /// search, an offer), never a match already made.
    fn leave(&mut self) -> impl Future<Output = anyhow::Result<()>> + Send;
}

/// The person's queue: one search at a time, one match per account.
pub struct Queue {
    player: Player,
    deck: Arc<Frozen>,
    /// How long one search may wait; `None` waits on.
    search: Option<Duration>,
    /// The search this source started, and when.
    waiting: Option<(Id, Instant)>,
}

impl Queue {
    /// Searches in `deck`'s format with it, each search for `search` at
    /// most.
    #[must_use]
    pub const fn new(player: Player, deck: Arc<Frozen>, search: Option<Duration>) -> Self {
        Self {
            player,
            deck,
            search,
            waiting: None,
        }
    }

    async fn join(&mut self) -> anyhow::Result<()> {
        let key = *Uuid::now_v7().as_bytes();
        let (format, digest) = (self.deck.deck.format, self.deck.digest);
        // The plain queue route answers `UpdateRequired` to this client: a
        // search joins through the replica route, as the native client's.
        let reply = self
            .player
            .call(move |account| {
                account.replica(SeatRequest::Join {
                    key,
                    format,
                    deck: digest,
                    // An AI never asks to meet an AI here: Central seats AI
                    // accounts through their offers, never a search.
                    ai: baylee_account::ranked::ai::AiOpponents::None,
                })
            })
            .await
            .map_err(|e| anyhow!("joining the {} queue: {e}", format.as_str()))?;
        if !matches!(reply, SeatResponse::Queue { .. }) {
            bail!("Central answered the search with something else");
        }
        tracing::info!(format = format.as_str(), "searching for an opponent");
        self.waiting = Some((key, Instant::now()));
        Ok(())
    }

    fn ours(&self, format: Format, key: Id) -> bool {
        format == self.deck.deck.format || self.waiting.is_some_and(|(k, _)| k == key)
    }
}

impl Assignments for Queue {
    async fn live(&mut self) -> anyhow::Result<Vec<Match>> {
        match status(&self.player).await? {
            // A match Central holds for the account: also one a runner that
            // restarted left, which it resumes.
            QueueState::Assigned { key, assignment } => {
                self.waiting = None;
                if assignment.deck != self.deck.digest {
                    bail!("Central holds a match for this account with another deck");
                }
                Ok(vec![Match {
                    key,
                    assignment,
                    deck: Arc::clone(&self.deck),
                }])
            }
            QueueState::Waiting { key, format, .. } if self.ours(format, key) => {
                let since = *self.waiting.get_or_insert((key, Instant::now()));
                if self.search.is_some_and(|wait| since.1.elapsed() >= wait) {
                    cancel(&self.player, key).await?;
                    self.waiting = None;
                    bail!("no opponent within {} s", since.1.elapsed().as_secs());
                }
                Ok(Vec::new())
            }
            QueueState::Waiting { key, .. } => {
                cancel(&self.player, key).await?;
                self.join().await?;
                Ok(Vec::new())
            }
            QueueState::Idle | QueueState::Finished { .. } => {
                self.join().await?;
                Ok(Vec::new())
            }
        }
    }

    async fn leave(&mut self) -> anyhow::Result<()> {
        if let QueueState::Waiting { key, .. } = status(&self.player).await? {
            cancel(&self.player, key).await?;
        }
        self.waiting = None;
        Ok(())
    }
}

/// Leaves the search `key`, if it is still waiting.
async fn cancel(player: &Player, key: Id) -> anyhow::Result<()> {
    player
        .call(move |account| account.ranked(Request::Cancel { key }).map(drop))
        .await
        .map_err(|e| anyhow!("leaving the search: {e}"))
}

async fn status(player: &Player) -> anyhow::Result<QueueState> {
    player
        .call(|account| match account.ranked(Request::Status)? {
            Response::Queue { state } => Ok(state),
            _ => Err(Error::Rejected),
        })
        .await
        .map_err(|e| anyhow!("reading the account's queue: {e}"))
}
