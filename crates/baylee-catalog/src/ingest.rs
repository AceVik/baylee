//! Filling the catalog: the bulk feed for everything, the API for stragglers.
//!
//! # Why bulk and not the API
//!
//! Scryfall's rate limit is ten requests a second (`docs/legal.md` §3), and
//! there are hundreds of thousands of printings. Walking them through the API
//! would take most of a day and would be rude; the bulk feed exists precisely
//! so nobody does that. The API is kept for the one case bulk cannot serve —
//! a card a client asks for that the catalog has never seen, usually because
//! the bulk snapshot predates the set.

use crate::scryfall::{BulkList, Card};
use crate::{Catalog, scryfall};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::io::{BufRead, BufReader};
use std::time::Duration;

/// User agent sent to Scryfall. Their terms ask for an identifying agent.
const USER_AGENT: &str = concat!("baylee/", env!("CARGO_PKG_VERSION"));

/// Pause between single-card API calls — well inside the published limit.
const RATE_LIMIT_PAUSE: Duration = Duration::from_millis(120);

/// How many printings are upserted per statement.
///
/// Postgres caps a statement at 65535 parameters; a face row binds twelve, so
/// this leaves a wide margin even for cards with several faces while still
/// making the round trip worth taking.
const BATCH: usize = 400;

/// Which bulk feed to ingest.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Feed {
    /// Every card in English (or its printed language when it has no English
    /// printing). ~78 MB compressed.
    Default,
    /// Every card in every language. ~392 MB compressed, and the only feed
    /// that makes a non-English client useful.
    AllLanguages,
}

impl Feed {
    /// The `type` field Scryfall lists this feed under.
    #[must_use]
    pub const fn scryfall_type(self) -> &'static str {
        match self {
            Self::Default => "default_cards",
            Self::AllLanguages => "all_cards",
        }
    }
}

/// Downloads a bulk feed and upserts every card in it.
///
/// The feed is streamed and decompressed on the fly: at 392 MB compressed and
/// several gigabytes decoded, holding it in memory is not an option, and
/// Scryfall serves it as JSONL so each line is one complete card.
///
/// # Errors
/// When Scryfall is unreachable, the feed is malformed, or the database
/// rejects a batch.
pub async fn bulk(catalog: &Catalog, feed: Feed) -> Result<usize> {
    let url = bulk_uri(feed)?;
    tracing::info!(%url, "downloading bulk feed");

    // The download is blocking (ureq) and the database is async, so the two
    // run as producer and consumer: parsing never waits for a round trip and
    // the upserts never wait for the network.
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<Card>>(4);
    let reader = std::thread::spawn(move || -> Result<()> {
        let response = ureq::get(&url)
            .header("User-Agent", USER_AGENT)
            .call()
            .context("downloading the bulk feed")?;
        let decoder = GzDecoder::new(response.into_body().into_reader());
        let mut batch = Vec::with_capacity(BATCH);
        for line in BufReader::new(decoder).lines() {
            let line = line.context("reading the bulk feed")?;
            let line = line.trim().trim_end_matches(',');
            // The feed is JSONL, but the first and last lines of the older
            // JSON form are brackets; skipping them costs nothing and makes
            // the ingest tolerant of either shape.
            if line.is_empty() || line == "[" || line == "]" {
                continue;
            }
            match serde_json::from_str::<Card>(line) {
                Ok(card) if card.is_storable() => batch.push(card),
                // A record this build cannot model is not worth failing the
                // whole ingest for — the next snapshot usually fixes it.
                Ok(_) => {}
                Err(err) => tracing::debug!(%err, "skipping unparseable record"),
            }
            if batch.len() >= BATCH && tx.blocking_send(std::mem::take(&mut batch)).is_err() {
                return Ok(());
            }
            if batch.capacity() < BATCH {
                batch.reserve(BATCH);
            }
        }
        if !batch.is_empty() {
            let _ = tx.blocking_send(batch);
        }
        Ok(())
    });

    let mut stored = 0usize;
    while let Some(batch) = rx.recv().await {
        stored += catalog.upsert(&batch).await?;
        if stored % 20_000 < BATCH {
            tracing::info!(stored, "ingesting");
        }
    }
    reader
        .join()
        .map_err(|_| anyhow::anyhow!("bulk reader thread panicked"))??;
    tracing::info!(stored, "ingest complete");
    Ok(stored)
}

/// Resolves a feed to its current download URL.
fn bulk_uri(feed: Feed) -> Result<String> {
    let list: BulkList = ureq::get(&format!("{}/bulk-data", scryfall::API))
        .header("User-Agent", USER_AGENT)
        .call()
        .context("listing bulk data")?
        .into_body()
        .read_json()
        .context("decoding the bulk-data list")?;
    list.data
        .into_iter()
        .find(|e| e.kind == feed.scryfall_type())
        .map(|e| e.jsonl_download_uri)
        .with_context(|| format!("Scryfall has no {} feed", feed.scryfall_type()))
}

/// Fetches one printing from the Scryfall API and stores it.
///
/// Used when a client asks for a card the catalog does not have. Blocking, so
/// callers in an async context run it on a blocking worker.
///
/// # Errors
/// When the request fails or the card does not exist.
pub fn fetch_one_blocking(id: &str) -> Result<Card> {
    std::thread::sleep(RATE_LIMIT_PAUSE);
    let card: Card = ureq::get(&format!("{}/cards/{id}", scryfall::API))
        .header("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("fetching card {id}"))?
        .into_body()
        .read_json()
        .context("decoding a card")?;
    Ok(card)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feeds_name_the_scryfall_types() {
        assert_eq!(Feed::Default.scryfall_type(), "default_cards");
        assert_eq!(Feed::AllLanguages.scryfall_type(), "all_cards");
    }

    /// The batch size has to stay inside Postgres' parameter cap with room for
    /// multi-face cards, which bind twelve parameters per face.
    #[test]
    fn a_batch_cannot_overflow_the_postgres_parameter_limit() {
        // Worst realistic case: every card in the batch has three faces.
        let faces_per_card = 3;
        let params = BATCH * faces_per_card * 12;
        assert!(params < 65535, "{params} parameters is over the cap");
    }
}
