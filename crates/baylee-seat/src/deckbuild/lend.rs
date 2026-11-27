//! A lent deck: one made without a model, for the pilots that cannot
//! build (the fair house levels, the trained net), deterministic given its
//! seed and the facts (`docs/proposals/ai-players.md` §"Decks").
//!
//! The deck generator (`baylee_cards::deckgen::build_synergistic`) builds
//! from the format's pool ([`super::pool::Pool`]) in the format's shape,
//! and the deck is held to the host's check ([`super::check`]); a deck the
//! check refuses is passed over for the next seed, up to [`TRIES`], and so is
//! a Standard deck of more than sixty cards (the generator makes one in four
//! longer, for its training variety). The archetype goes round with the
//! seed, so seeds in a row lend different kinds of deck.
//!
//! The precons (`docs/precons.md`) are not drawn on: of the 89 this build
//! plays, none is a hundred-card singleton or Commander deck, and the
//! sixty-card ones are lists of the 1990s, which no Standard host admits.

use super::check::{Checker, Rows, Verdict};
use super::pool::{Pool, letters};
use crate::ranked::Frozen;
use baylee_account::ranked::Format;
use baylee_cards::deckgen::{Archetype, DeckBuildRequest, Generated, build_synergistic};
use baylee_cards::formats::{CONSTRUCTED_MIN, Shape};
use baylee_core::ids::CardIndex;
use std::collections::BTreeMap;

/// The most seeds tried before lending gives up.
pub const TRIES: u64 = 64;

/// A lent deck.
#[derive(Debug)]
pub struct Lent {
    /// The deck, as its file holds it.
    pub rows: Rows,
    /// It frozen, as the runner offers it.
    pub frozen: Frozen,
    /// The seed it was asked with.
    pub seed: u64,
    /// The generator's seed that made the deck that passed.
    pub made_with: u64,
    /// The generator's own name for it (shape, archetype, colours, seed).
    pub generated: String,
    /// The seeds whose deck the check refused, and the first complaint of
    /// each.
    pub passed_over: Vec<(u64, String)>,
}

/// The shape a ranked format's deck has.
#[must_use]
pub const fn shape(format: Format) -> Shape {
    match format {
        Format::Standard => Shape::Constructed,
        Format::Highlander => Shape::Highlander,
        Format::Commander => Shape::Commander,
    }
}

/// A seed's `n`-th try (`SplitMix64`), so neighbouring seeds share no try.
const fn mix(seed: u64, n: u64) -> u64 {
    let mut z = seed.wrapping_add(n.wrapping_add(1).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A deck of `format` lent for `seed`, from `pool`, held to `checker`'s
/// check at `now`.
///
/// # Errors
/// When the pool is empty, or no deck of [`TRIES`] seeds passes: the
/// sentence names the last generator fault or complaint.
pub fn lend(
    checker: &Checker<'_>,
    pool: &Pool,
    format: Format,
    seed: u64,
    now: &str,
) -> Result<Lent, String> {
    if pool.is_empty() {
        return Err(format!(
            "the {} pool is empty under the facts from {}: no card is both played \
             rules-complete here and legal there",
            format.as_str(),
            checker.facts().gateway
        ));
    }
    let indices: Vec<CardIndex> = pool.cards().iter().map(|c| c.index).collect();
    let generator =
        baylee_cards::deckgen::Pool::from_cards(indices, &std::collections::BTreeSet::new());
    let mut passed_over = Vec::new();
    let mut last = String::new();
    for n in 0..TRIES {
        let archetype = Archetype::ALL[usize::try_from((seed.wrapping_add(n)) % 4).unwrap_or(0)];
        let made_with = mix(seed, n);
        let generated = match build_synergistic(
            &generator,
            &DeckBuildRequest::new(shape(format), archetype, made_with),
        ) {
            Ok(generated) => generated,
            Err(e) => {
                last = format!("the generator: {}", e.0);
                passed_over.push((made_with, last.clone()));
                continue;
            }
        };
        // The generator makes one constructed deck in four 61 to 75 cards
        // for its training variety; a lent deck is the sixty a player
        // brings, so such a seed is passed over.
        if format == Format::Standard && generated.main.len() > CONSTRUCTED_MIN {
            last = format!(
                "the generator made {} cards, and a lent Standard deck is {CONSTRUCTED_MIN}",
                generated.main.len()
            );
            passed_over.push((made_with, last.clone()));
            continue;
        }
        let rows = rows_of(&generated, format);
        match checker.check(format, &rows, now) {
            Verdict {
                frozen: Some(frozen),
                ..
            } => {
                return Ok(Lent {
                    rows,
                    frozen,
                    seed,
                    made_with,
                    generated: generated.name.clone(),
                    passed_over,
                });
            }
            Verdict { complaints, .. } => {
                last = complaints.first().cloned().unwrap_or_default();
                passed_over.push((made_with, last.clone()));
            }
        }
    }
    Err(format!(
        "no {} deck of {TRIES} tries passed the host's check; the last: {last}",
        format.as_str()
    ))
}

/// `generated` as a deck file's rows, named for what it is.
fn rows_of(generated: &Generated, format: Format) -> Rows {
    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    for card in &generated.main {
        *counts.entry(row_name(*card)).or_default() += 1;
    }
    let archetype = match generated.archetype {
        Archetype::Aggro => "Aggro",
        Archetype::Midrange => "Midrange",
        Archetype::Control => "Control",
        Archetype::Ramp => "Ramp",
    };
    let format_name = match format {
        Format::Standard => "Standard",
        Format::Highlander => "Highlander",
        Format::Commander => "Commander",
    };
    Rows {
        name: format!("{} {archetype} {format_name}", letters(generated.colors)),
        main: counts
            .into_iter()
            .map(|(name, count)| format!("{count} {name}"))
            .collect(),
        sideboard: Vec::new(),
        commanders: generated.leaders.iter().map(|c| row_name(*c)).collect(),
    }
}

/// The name a deck row spells `card` by.
fn row_name(card: CardIndex) -> String {
    baylee_cards::decks::whole_name(card)
        .map(str::to_string)
        .or_else(|| baylee_cards::by_index(card).map(|def| def.name().to_string()))
        .unwrap_or_default()
}
