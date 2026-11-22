//! Frozen ranked registrations, shared by the originating client and the host.
//! Only the host can accept one for play, using its trusted legality catalog.

use std::collections::BTreeMap;

use baylee_core::deckrow;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::decks::{DeckCard, LoadedDeck};
use crate::formats::{self, PoolFacts, Shape, Standing};

#[cfg(test)]
mod tests;

/// The actual engine deck, its zone-sensitive commitment, and the copy counts
/// used for Elo. A commander is a different role from a library card.
#[derive(Debug)]
pub struct Registration {
    /// Exactly the resolved deck used to compute the commitment.
    pub deck: LoadedDeck,
    /// Versioned SHA-256 commitment. Alternate physical prints can have rules
    /// effects, so their UUID and per-copy choices must also be committed.
    pub digest: [u8; 32],
    /// Oracle identity, commander role, and multiplicity, in canonical order.
    pub pool: BTreeMap<(Uuid, bool), u16>,
}

/// Loads the three lists in the form returned by the gateway's deck route.
/// Leader rows already present in the main list move to the command zone.
///
/// # Errors
/// Refuses malformed, unavailable or oversized decks and unknown formats.
pub fn registration(
    format: &str,
    cards: &[String],
    sideboard: &[String],
    commanders: &[String],
) -> Result<Registration, String> {
    let format_byte = match format {
        "standard" => 0,
        "highlander" => 1,
        "commander" => 2,
        _ => return Err("unknown ranked format".into()),
    };
    if cards.is_empty()
        || cards.len() > 250
        || sideboard.len() > formats::SIDEBOARD_MAX
        || commanders.len() > 2
        || commanders.iter().any(|name| name.len() > 500)
    {
        return Err("invalid ranked deck size".into());
    }
    let mut main = zone(cards, 250)?;
    let sideboard = zone(sideboard, formats::SIDEBOARD_MAX)?;
    let mut leaders = Vec::with_capacity(commanders.len());
    for name in commanders {
        let index = crate::decks::by_name(name).ok_or("unknown ranked commander")?;
        if leaders.iter().any(|c: &DeckCard| c.index == index) {
            return Err("a commander can only be registered once".into());
        }
        let card = main
            .iter()
            .position(|c| c.index == index)
            .map_or_else(|| DeckCard::plain(index), |at| main.remove(at));
        leaders.push(card);
    }
    let mut deck = LoadedDeck {
        name: String::new(),
        main,
        sideboard,
        commanders: leaders,
    };
    if deck.main.len() + deck.sideboard.len() + deck.commanders.len() > 250 {
        return Err(
            "ranked decks hold at most 250 cards including sideboard and commanders".into(),
        );
    }
    let mut hash = Sha256::new();
    hash.update(b"baylee-ranked-registration-v1\0");
    hash.update([format_byte]);
    let mut pool = BTreeMap::new();
    for (zone, copies) in [
        (0, &mut deck.main),
        (1, &mut deck.sideboard),
        (2, &mut deck.commanders),
    ] {
        // Also canonicalize the engine's starting library: reordering the
        // submitted rows must not select a different shuffle for the same seed.
        copies.sort_by_cached_key(|copy| {
            (
                crate::by_index(copy.index).map(|def| def.oracle_id),
                copy.print.scryfall_id,
                copy.choices.colors().bits(),
            )
        });
        hash.update([zone]);
        hash.update(
            u16::try_from(copies.len())
                .map_err(|_| "deck too large")?
                .to_be_bytes(),
        );
        for copy in copies.iter() {
            let def = crate::by_index(copy.index).ok_or("unknown ranked card")?;
            let oracle = Uuid::parse_str(def.oracle_id).map_err(|_| "missing Oracle identity")?;
            *pool.entry((oracle, zone == 2)).or_insert(0) += 1;
            hash.update(oracle.as_bytes());
            hash.update(copy.print.scryfall_id.as_bytes());
            hash.update([copy.choices.colors().bits()]);
        }
    }
    Ok(Registration {
        deck,
        digest: hash.finalize().into(),
        pool,
    })
}

fn zone(rows: &[String], limit: usize) -> Result<Vec<DeckCard>, String> {
    let mut copies = Vec::new();
    for line in rows {
        if line.len() > 4096 {
            return Err("ranked deck row too long".into());
        }
        let row = deckrow::parse(line).map_err(|error| error.to_string())?;
        let count = usize::try_from(row.count).map_err(|_| "invalid ranked card count")?;
        if count == 0 || count > limit - copies.len() {
            return Err("invalid ranked card count".into());
        }
        if row
            .print
            .scryfall_id
            .as_deref()
            .is_some_and(|id| Uuid::parse_str(id).is_err())
        {
            return Err("invalid printing UUID".into());
        }
        let index = crate::decks::by_name(&row.name).ok_or("unknown ranked card")?;
        crate::print_rules::validate_choices(index, row.print.choices)?;
        copies.extend(std::iter::repeat_n(
            DeckCard::chosen(index, &row.print),
            count,
        ));
    }
    Ok(copies)
}

/// Applies catalog legality and all shape checks to the deck the host will use.
/// Baylee Highlander uses its own 100-card singleton shape and the Vintage
/// paper card pool, for which the catalog has per-card legality verdicts.
///
/// # Errors
/// Reports an illegal deck, unknown catalog verdict or incomplete card script.
pub fn admit(format: &str, deck: &LoadedDeck, facts: &PoolFacts) -> Result<(), String> {
    let catalog_key = match format {
        "standard" => "standard",
        "commander" => "commander",
        "highlander" => "vintage",
        _ => return Err("unknown ranked format".into()),
    };
    let report = formats::validate_loaded(
        deck,
        formats::Rules {
            format: catalog_key,
            facts: Some(facts),
            list: None,
        },
    );
    if report.standing() != Standing::Legal {
        return Err("deck has illegal cards, an invalid shape or missing format verdicts".into());
    }
    if format == "highlander" {
        let indices = |cards: &[DeckCard]| cards.iter().map(|c| c.index).collect::<Vec<_>>();
        if !formats::shape_problems(
            Shape::Highlander,
            &indices(&deck.main),
            &indices(&deck.sideboard),
            &indices(&deck.commanders),
        )
        .is_empty()
        {
            return Err("invalid Highlander deck".into());
        }
    }
    for card in deck
        .main
        .iter()
        .chain(&deck.sideboard)
        .chain(&deck.commanders)
    {
        let def = crate::by_index(card.index).ok_or("unknown ranked card")?;
        if !matches!(
            def.coverage,
            crate::dsl::Coverage::Implemented | crate::dsl::Coverage::Generated
        ) {
            return Err(format!(
                "{} has incomplete ranked rules coverage",
                def.faces[0].name
            ));
        }
    }
    if format == "commander" {
        let allowed = deck
            .commanders
            .iter()
            .filter_map(|c| crate::print_rules::identity(c.index, c.choices))
            .fold(
                baylee_core::color::ColorSet::EMPTY,
                baylee_core::color::ColorSet::union,
            );
        if deck
            .main
            .iter()
            .any(|c| !c.choices.colors().difference(allowed).is_empty())
        {
            return Err("circled colors exceed commander identity".into());
        }
    }
    Ok(())
}
