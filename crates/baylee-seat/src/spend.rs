//! The bridge's side of the spend book
//! ([`baylee_client_core::llmseat::ledger`], `docs/llm-seat.md` §"The spend
//! book"): a game reserves what it may spend before it sits down, plays
//! under that as a hard limit, and settles with its bill when it is over.
//!
//! [`Booked`] settles when it is dropped, so the game's end, an error on
//! the way out, a panic and a stopped process (the bridge drops it on
//! ctrl-c) all settle the one way. A bridge killed outright never settles,
//! and its reservation counts in full.

use crate::llm::{Settings, Tally};
use baylee_client_core::llmseat::ledger::{Ask, Book, Budget, Grant, Moment};
use baylee_client_core::llmseat::{Caps, FIRST_CALL_BYTES};
use std::sync::{Arc, Mutex, PoisonError};

/// The player's clock now: the Unix time, and its offset from UTC where
/// the platform says (on unix, the zone the process started in). The one
/// place the spend book's time is read.
#[must_use]
pub fn now() -> Moment {
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        });
    let offset = time::OffsetDateTime::from_unix_timestamp(unix)
        .ok()
        .and_then(|at| time::UtcOffset::local_offset_at(at).ok())
        .map(time::UtcOffset::whole_seconds);
    Moment { unix, offset }
}

/// A game's reservation in the spend book, settled when it is dropped.
#[derive(Debug)]
pub struct Booked {
    book: Book,
    grant: Grant,
    priced: bool,
    tally: Option<Arc<Mutex<Tally>>>,
    settled: bool,
}

/// Reserves in `book` what the game `settings` plays may spend under
/// `caps` at `now`, and makes `settings` a hard limit at what was granted:
/// the game's own budget, or less where less is left of the day or the
/// month.
///
/// # Errors
/// The book's refusal (too little left of a cap for one call, a model with
/// no price under a dollar cap alone, an unreadable book), or a game whose
/// token budget cannot hold one call.
pub fn reserve(
    book: &Book,
    caps: &Caps,
    settings: &mut Settings,
    profile: Option<&str>,
    now: Moment,
) -> Result<Booked, String> {
    let first = settings.worst(FIRST_CALL_BYTES as usize);
    if settings.spend_tokens < first.tokens {
        return Err(format!(
            "a game's budget of {} tokens cannot hold one call of «{}», which may take up to {}: \
             raise the game's token budget",
            settings.spend_tokens, settings.model, first.tokens
        ));
    }
    let (game, floor) = match (settings.price, settings.spend_usd, first.usd) {
        (Some(_), Some(usd), Some(floor)) => (Budget::Usd(usd), Budget::Usd(floor)),
        _ => (
            Budget::Tokens(settings.spend_tokens),
            Budget::Tokens(first.tokens),
        ),
    };
    let ask = Ask {
        profile,
        model: &settings.model,
        game,
        floor,
    };
    let grant = book.update(|ledger| ledger.reserve(caps, &ask, now))?;
    let priced = match grant.budget {
        Budget::Usd(usd) => {
            settings.spend_usd = Some(usd);
            true
        }
        Budget::Tokens(tokens) => {
            settings.spend_tokens = tokens;
            false
        }
    };
    settings.hard_limit = true;
    Ok(Booked {
        book: book.clone(),
        grant,
        priced,
        tally: None,
        settled: false,
    })
}

impl Booked {
    /// What was granted.
    #[must_use]
    pub const fn grant(&self) -> Grant {
        self.grant
    }

    /// Settles with what `tally` counts, once the mind that keeps it plays.
    pub fn watch(&mut self, tally: Arc<Mutex<Tally>>) {
        self.tally = Some(tally);
    }

    /// Settles the reservation at `now` with what the game may have cost:
    /// what the provider counted, and at their worst the calls still out
    /// and those whose bill is unknown; nothing, when no mind was ever
    /// watched. Once only; later calls do nothing.
    ///
    /// # Errors
    /// Why the book could not be written; the reservation then counts in
    /// full.
    pub fn settle(&mut self, now: Moment) -> Result<(), String> {
        if self.settled {
            return Ok(());
        }
        self.settled = true;
        let tally = self
            .tally
            .as_ref()
            .map(|t| t.lock().unwrap_or_else(PoisonError::into_inner).clone())
            .unwrap_or_default();
        let usd = self.priced.then(|| tally.spend_usd().unwrap_or(0.0));
        let id = self.grant.id;
        self.book
            .update(|ledger| ledger.settle(id, usd, tally.spend_tokens(), now))
    }
}

impl Drop for Booked {
    fn drop(&mut self) {
        if let Err(why) = self.settle(now()) {
            eprintln!("the game's reservation stays counted in full: {why}");
        }
    }
}

#[cfg(test)]
mod tests;
