//! One ranked match, from its assignment to Central's receipt: the seat
//! played through a [`RankedLink`] by the bridge, then the settled result
//! read back from the queue.

use super::{Frozen, LinkOptions, Pilot, Player, RankedLink};
use crate::bridge::{self, PlayOptions, Played};
use crate::seat::{BridgeConfig, Labelled, SeatCore};
use crate::transcript::Transcript;
use anyhow::Context as _;
use baylee_account::ranked::{Assignment, Id, Outcome, QueueState, Request, Response};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// A match Central assigned the account: its queue entry and its seat.
#[derive(Clone, Debug)]
pub struct Match {
    /// The account's queue entry the match was made from.
    pub key: Id,
    /// The account's seat in it.
    pub assignment: Assignment,
    /// The deck it was made with.
    pub deck: Arc<Frozen>,
}

/// How a match is played.
#[derive(Clone, Debug)]
pub struct MatchOptions {
    /// The bridge's pacing.
    pub play: PlayOptions,
    /// The link's waiting and renewal.
    pub link: LinkOptions,
    /// How long Central may take to settle the result once the game is over.
    pub settle: Duration,
    /// Where each match's transcript goes (`<game>-seat<n>.jsonl`).
    pub transcripts: Option<PathBuf>,
}

impl Default for MatchOptions {
    fn default() -> Self {
        Self {
            play: PlayOptions::default(),
            link: LinkOptions::default(),
            settle: Duration::from_secs(120),
            transcripts: None,
        }
    }
}

/// Central's receipt for a finished match.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    /// The engine's result.
    pub outcome: Outcome,
    /// The account's seat.
    pub seat: u8,
    /// Its rating after the match.
    pub elo: i32,
    /// The change.
    pub delta: i32,
}

impl Receipt {
    /// Whether the account won, lost or neither.
    #[must_use]
    pub const fn won(&self) -> Option<bool> {
        match self.outcome {
            Outcome::FirstWins => Some(self.seat == 0),
            Outcome::SecondWins => Some(self.seat == 1),
            Outcome::Draw => None,
        }
    }
}

/// How one match went.
#[derive(Debug)]
pub struct MatchReport {
    /// Central's match.
    pub match_id: Id,
    /// The seat's game, as the bridge played it.
    pub played: Played,
    /// Central's receipt, once it settled; `None` when it did not within
    /// [`MatchOptions::settle`] (a game closed without a result settles none).
    pub receipt: Option<Receipt>,
    /// Admissions renewed during the game.
    pub renewals: u32,
}

/// Plays `game` for `player` with its deck and `pilot` to its end, then
/// reads Central's receipt.
///
/// Takes only shared handles, so several matches of one account can run
/// at once where Central assigns them.
///
/// # Errors
/// When the seat could not be taken or left the table.
pub async fn play_match(
    player: &Player,
    game: Match,
    pilot: &Pilot,
    options: &MatchOptions,
) -> anyhow::Result<MatchReport> {
    let match_id = game.assignment.match_id;
    let deck = game.deck;
    let mut link = RankedLink::new(
        player.clone(),
        game.key,
        game.assignment,
        Arc::clone(&deck),
        options.link.clone(),
    );
    let mind = (pilot.make)();
    let config = BridgeConfig {
        think: pilot.think,
        house: pilot.house,
        labelled: Labelled::Account,
        ..BridgeConfig::default()
    };
    let core = SeatCore::new(config, deck.bridge.list.clone(), mind.disclosure())
        .with_mind(pilot.declared.clone());
    let mut transcript = match &options.transcripts {
        Some(dir) => {
            let path = dir.join(format!(
                "ranked-{}-{}.jsonl",
                Uuid::from_bytes(match_id),
                player.name()
            ));
            Transcript::file(&path).with_context(|| format!("open {}", path.display()))?
        }
        None => Transcript::none(),
    };
    let played = bridge::play(&mut link, core, mind, &mut transcript, &options.play).await?;
    transcript.write_value(&serde_json::json!({ "summary": played.stats }));
    transcript.flush();
    let renewals = link.renewals();
    drop(link);
    let receipt = settled(player, match_id, options.settle).await;
    Ok(MatchReport {
        match_id,
        played,
        receipt,
        renewals,
    })
}

/// Central's receipt for `match_id`, read from the account's queue until it
/// settles or `patience` runs out.
async fn settled(player: &Player, match_id: Id, patience: Duration) -> Option<Receipt> {
    let until = tokio::time::Instant::now() + patience;
    loop {
        let state = player
            .call(|account| match account.ranked(Request::Status)? {
                Response::Queue { state } => Ok(state),
                _ => Err(baylee_account::Error::Rejected),
            })
            .await;
        match state {
            Ok(QueueState::Finished {
                match_id: id,
                outcome,
                seat,
                elo,
                delta,
                ..
            }) if id == match_id => {
                return Some(Receipt {
                    outcome,
                    seat,
                    elo,
                    delta,
                });
            }
            Ok(QueueState::Assigned { assignment, .. }) if assignment.match_id == match_id => {}
            Ok(_) => return None,
            Err(e) => tracing::info!(error = %e, "Central could not say the result yet"),
        }
        if tokio::time::Instant::now() >= until {
            return None;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
