//! One ranked match of an AI account, from its assignment to Central's
//! result: the seat played through a [`RankedLink`] by the account's
//! [`Pilot`] (and by the house fallback, where the pilot hands it over),
//! then the account's standing read back.

use super::{Frozen, LinkOptions, Pilot, Piloted, Player, RankedLink, Seat};
use crate::bridge::{PlayOptions, Played};
use crate::transcript::Transcript;
use anyhow::{Context as _, bail};
use baylee_account::ranked::{
    Assignment, Format, Id, Outcome, ProfileTarget, Request, Response, Standing, Track, ai,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// A match Central made for the account from its offer.
#[derive(Clone, Debug)]
pub struct Match {
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
    /// Where each match's transcript goes
    /// (`ranked-<match>-<account name>.jsonl`).
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

/// Central's result for a finished match: as its `Work` lists the account's
/// finished matches, else inferred from how the account's standing moved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Receipt {
    /// The engine's result, as Central counted it.
    pub outcome: Outcome,
    /// The account's seat.
    pub seat: u8,
    /// Its rating after the match.
    pub elo: i32,
    /// The change; `None` where the account had no standing before.
    pub delta: Option<i32>,
    /// Whether it was inferred from the standing, Central's list of finished
    /// matches not naming the match.
    pub inferred: bool,
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
    /// The seat's game, as its pilot played it (and the house after it,
    /// where it handed over).
    pub played: Played,
    /// Why the pilot handed the seat to the house, where it did.
    pub handed_over: Option<String>,
    /// Central's result, once it settled; `None` when it did not within
    /// [`MatchOptions::settle`], or when another of the account's matches
    /// in the format settled meanwhile and the standing cannot tell them
    /// apart.
    pub receipt: Option<Receipt>,
    /// Admissions renewed during the game.
    pub renewals: u32,
    /// The match's format.
    pub format: Format,
    /// The commitment of the deck it was played with.
    pub deck: [u8; 32],
    /// The opponent's AI label, where the opponent is an AI; never a
    /// person's name.
    pub opponent: Option<String>,
}

/// Plays `game` for `player` with `pilot` to its end, `fallback` taking the
/// seat where the pilot hands it over, then reads Central's result.
///
/// Takes only shared handles, so several matches of one account run at
/// once.
///
/// # Errors
/// When the seat could not be taken or left the table.
pub async fn play_match(
    player: &Player,
    game: Match,
    pilot: &dyn Pilot,
    fallback: &dyn Pilot,
    options: &MatchOptions,
) -> anyhow::Result<MatchReport> {
    let (match_id, format) = (game.assignment.match_id, game.assignment.format);
    let opponent = game
        .assignment
        .opponent_ai
        .as_ref()
        .map(|badge| badge.label.clone());
    let before = standing(player, format).await;
    let mut link = RankedLink::new(
        player.clone(),
        game.assignment,
        Arc::clone(&game.deck),
        options.link.clone(),
    );
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
    let mut handed_over = None;
    let flown = pilot
        .fly(Seat {
            link: &mut link,
            deck: &game.deck,
            transcript: &mut transcript,
            play: &options.play,
        })
        .await?;
    let played = match flown {
        Piloted::Played(played) => *played,
        Piloted::HandOver(why) => {
            tracing::warn!(why = %why, "the pilot handed the seat to the house");
            transcript.write_value(&serde_json::json!({
                "handed_over": { "why": why, "to": fallback.profile() }
            }));
            handed_over = Some(why);
            let flown = fallback
                .fly(Seat {
                    link: &mut link,
                    deck: &game.deck,
                    transcript: &mut transcript,
                    play: &options.play,
                })
                .await?;
            match flown {
                Piloted::Played(played) => *played,
                Piloted::HandOver(why) => bail!("the house fallback handed the seat over: {why}"),
            }
        }
    };
    transcript.write_value(&serde_json::json!({ "summary": played.stats }));
    transcript.flush();
    let receipt = settled(player, &link, before, options.settle).await;
    if let Some(r) = &receipt {
        // Where the result was inferred, the record says so.
        transcript.write_value(&serde_json::json!({ "receipt": {
            "outcome": format!("{:?}", r.outcome), "seat": r.seat, "elo": r.elo,
            "delta": r.delta,
            "source": if r.inferred { "inferred from the account's public standing" } else { "Central" },
        }}));
        transcript.flush();
    }
    Ok(MatchReport {
        match_id,
        played,
        handed_over,
        receipt,
        renewals: link.renewals(),
        format,
        deck: game.deck.digest,
        opponent,
    })
}

/// The account's standing on the player ladder of `format`, as its public
/// profile says; `None` where it has none yet or Central cannot say.
async fn standing(player: &Player, format: Format) -> Option<Standing> {
    let account = player.account();
    player
        .call(move |a| {
            match a.ranked(Request::Profile {
                target: ProfileTarget::Account(account),
                season: None,
            })? {
                Response::Profile { formats, .. } => Ok(formats
                    .into_iter()
                    .find(|s| s.format == format && s.track == Track::Player)),
                _ => Err(baylee_account::Error::Rejected),
            }
        })
        .await
        .unwrap_or_else(|e| {
            tracing::info!(error = %e, "Central could not say the account's standing");
            None
        })
}

/// Central's result for the link's match: the one its `Work` lists among
/// the account's finished matches (`ai::Finished`), asked until it does or
/// `patience` runs out; else, where Central's list does not name it, the
/// account's standing read once against `before`, marked inferred.
async fn settled(
    player: &Player,
    link: &RankedLink,
    before: Option<Standing>,
    patience: Duration,
) -> Option<Receipt> {
    let (match_id, format, seat) = (
        link.assignment().match_id,
        link.assignment().format,
        link.assignment().seat,
    );
    let until = tokio::time::Instant::now() + patience;
    loop {
        if let Some(receipt) = from_work(player, match_id).await {
            return Some(receipt);
        }
        if tokio::time::Instant::now() >= until {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    tracing::info!("Central's finished matches do not name this one: the standing says the result");
    inferred(player, format, seat, before).await
}

/// The match `match_id` among the account's finished ones, as Central's
/// `Work` lists them.
async fn from_work(player: &Player, match_id: Id) -> Option<Receipt> {
    let reply = player
        .call(|account| account.ai(ai::Request::Work))
        .await
        .ok()?;
    let ai::Response::Work { finished, .. } = reply else {
        return None;
    };
    let done = finished.into_iter().find(|f| f.match_id == match_id)?;
    // The first seat won where this seat won and is the first, or lost and
    // is the second.
    let outcome = match done.result {
        ai::MatchResult::Draw => Outcome::Draw,
        result if (result == ai::MatchResult::Win) == (done.seat == 0) => Outcome::FirstWins,
        _ => Outcome::SecondWins,
    };
    Some(Receipt {
        outcome,
        seat: done.seat,
        elo: done.elo,
        delta: Some(done.delta),
        inferred: false,
    })
}

/// The account's standing in `format` read once: one game more than
/// `before`, its result inferred from what moved.
async fn inferred(
    player: &Player,
    format: Format,
    seat: u8,
    before: Option<Standing>,
) -> Option<Receipt> {
    let after = standing(player, format).await?;
    let (games, wins, losses) = before
        .as_ref()
        .map_or((0, 0, 0), |b| (b.games, b.wins, b.losses));
    if after.games.checked_sub(games) != Some(1) {
        return None;
    }
    let won = if after.wins > wins {
        Some(true)
    } else if after.losses > losses {
        Some(false)
    } else {
        None
    };
    // The first seat won where this seat won and is the first, or lost and
    // is the second.
    let outcome = match won {
        None => Outcome::Draw,
        Some(won) if won == (seat == 0) => Outcome::FirstWins,
        Some(_) => Outcome::SecondWins,
    };
    Some(Receipt {
        outcome,
        seat,
        elo: after.elo,
        delta: before.map(|b| after.elo - b.elo),
        inferred: true,
    })
}
