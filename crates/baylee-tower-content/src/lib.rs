//! Reviewed content adapter. The durable Tower authority never links the card pool.
#![forbid(unsafe_code)]
#![warn(missing_docs)]

use baylee_core::{ids::CardIndex, preset::SeatSpec};
use baylee_tower::cards::{Card, Catalog, Collection, Formation, RunCard, Stack};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

mod expeditions;
mod house;
mod presentation;
mod starters;
pub use expeditions::{Workshop, expeditions};
pub use house::{Ladder, PATH as HOUSE_PATH, check as check_ladder, development_house, ladder};
pub use presentation::runic;

#[cfg(test)]
use baylee_cards_pool as _;
#[cfg(test)]
mod tests;

/// An operator binds frozen content to the publication's exact native executor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Content {
    /// Frozen card identity and Commander metadata, derived from the compiled registry.
    pub catalog: Catalog,
    /// Distinct, independently legal opening collections, frozen at publication.
    pub starters: Vec<Collection>,
}

/// A content failure prevents publication or native admission.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A named starter card must exist and have its own played test.
    #[error("starter card is absent or not fully tested: {0}")]
    Card(&'static str),
    /// The immutable catalog disagrees with the native registry.
    #[error("Tower card metadata does not match the native registry")]
    Metadata,
    /// The generation-four deck contract was violated.
    #[error("invalid forty-card Commander formation")]
    Formation,
    /// The shared curve/synergy/mana-base builder could not make a starter.
    #[error("starter deck generation failed: {0}")]
    Generation(#[from] baylee_cards::deckgen::DeckGenError),
    /// The house ladder could not be built: a defect, never a balance result.
    #[error("house ladder: {0}")]
    House(&'static str),
    /// The run machine refused a scripted house step.
    #[error("house ladder step refused: {0}")]
    Refused(String),
}

/// Build the five opening archetypes. No random deck changes occur at enrollment.
/// # Errors
/// An unavailable/untested card or an invalid starter is a publication failure.
pub fn standard() -> Result<Content, Error> {
    let mut cards = BTreeMap::new();
    for name in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        let card = named(name)?;
        cards.insert(card.card, card);
    }
    let mut collections = Vec::with_capacity(starters::ALL.len());
    for template in starters::ALL {
        let commander = named(template.commander)?;
        cards.insert(commander.card, commander);
        let mut owned = vec![Stack {
            card: commander.card,
            copies: 1,
        }];
        let mut deck = Vec::with_capacity(template.spells.len() + template.lands.len());
        for name in template.spells {
            let card = named(name)?;
            cards.insert(card.card, card);
            let entry = Stack {
                card: card.card,
                copies: 1,
            };
            owned.push(entry);
            deck.push(entry);
        }
        for &(name, copies) in template.lands {
            deck.push(Stack {
                card: named(name)?.card,
                copies,
            });
        }
        deck.sort_by_key(|row| row.card);
        owned.sort_by_key(|row| row.card);
        collections.push(Collection {
            formation: Some(Formation {
                commander: commander.card,
                sideboard: Vec::new(),
            }),
            owned,
            deck,
        });
    }
    let catalog = Catalog {
        cards: cards.into_values().collect(),
    };
    catalog
        .validate_starters(&collections)
        .map_err(|_| Error::Formation)?;
    Ok(Content {
        catalog,
        starters: collections,
    })
}

fn named(name: &'static str) -> Result<Card, Error> {
    let index = baylee_cards::decks::by_name(name).ok_or(Error::Card(name))?;
    metadata(index).ok_or(Error::Card(name))
}

fn metadata(index: CardIndex) -> Option<Card> {
    let definition = baylee_cards::by_index(index)?;
    if definition.coverage != baylee_cards::dsl::Coverage::Implemented {
        return None;
    }
    let basic_land = baylee_cards::formats::is_basic_land(index);
    let commander =
        baylee_cards::decks::leader_of(index).is_some_and(|leader| leader.eligible_alone);
    Some(Card {
        card: index,
        price: if basic_land { 0 } else { 4 },
        basic_land,
        run: Some(RunCard {
            identity: definition.color_identity,
            commander,
            presentation: None,
        }),
    })
}

/// Refuse operator metadata that could disguise a nonbasic, off-color or untested card.
/// # Errors
/// Invalid structure or facts differing from this exact compiled card registry.
pub fn validate_catalog(catalog: &Catalog) -> Result<(), Error> {
    catalog.validate().map_err(|_| Error::Metadata)?;
    for card in &catalog.cards {
        let actual = metadata(card.card).ok_or(Error::Metadata)?;
        let mut rules = card.run;
        if let Some(rules) = &mut rules
            && let Some(art) = rules.presentation.take()
            && art != presentation::of(card.card)?
        {
            return Err(Error::Metadata);
        }
        if rules != actual.run || card.basic_land != actual.basic_land {
            return Err(Error::Metadata);
        }
    }
    Ok(())
}

/// Independently verify the actual executable deck, without trusting operator labels.
/// Ownership is enforced by the authority; native execution checks the real rules.
/// # Errors
/// Wrong zones, commander, color identity, unsupported card or singleton violation.
pub fn validate_seat(seat: &SeatSpec) -> Result<(), Error> {
    if seat.commanders.len() != 1 || seat.deck.len() != 39 || seat.sideboard.len() > 5 {
        return Err(Error::Formation);
    }
    let mut catalog = Catalog { cards: Vec::new() };
    for name in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
        catalog.cards.push(named(name)?);
    }
    let mut owned = BTreeMap::new();
    for row in seat
        .commanders
        .iter()
        .chain(&seat.deck)
        .chain(&seat.sideboard)
    {
        // This generation publishes only the reference printing and no per-copy
        // construction choices. Do not accept extra rules the catalog did not bind.
        if row.rule_print.is_some()
            || row.choices != baylee_core::card_metadata::DeckChoices::default()
        {
            return Err(Error::Metadata);
        }
        let card = metadata(row.card).ok_or(Error::Metadata)?;
        if !catalog.cards.iter().any(|entry| entry.card == card.card) {
            catalog.cards.push(card);
        }
        if !card.basic_land {
            // Ownership of one copy; duplicate use is independently refused by
            // the complete formation check across main/commander/sideboard.
            owned.insert(
                card.card,
                Stack {
                    card: card.card,
                    copies: 1,
                },
            );
        }
    }
    let count = |entries: &[baylee_core::preset::DeckEntry]| {
        let mut rows = BTreeMap::<CardIndex, u8>::new();
        for entry in entries {
            *rows.entry(entry.card).or_default() += 1;
        }
        rows.into_iter()
            .map(|(card, copies)| Stack { card, copies })
            .collect()
    };
    Collection {
        formation: Some(Formation {
            commander: seat.commanders[0].card,
            sideboard: count(&seat.sideboard),
        }),
        owned: owned.into_values().collect(),
        deck: count(&seat.deck),
    }
    .validate(&catalog)
    .map_err(|_| Error::Formation)
}

/// A card's reviewed catalog entry for the current generation: native
/// metadata and its frozen presentation. For calibration candidates only;
/// a publication takes its catalog from [`runic`].
/// # Errors
/// An unsupported card or one without a reviewed printing.
pub fn run_card(index: CardIndex) -> Result<Card, Error> {
    let mut card = metadata(index).ok_or(Error::Metadata)?;
    card.run.as_mut().ok_or(Error::Metadata)?.presentation = Some(presentation::of(index)?);
    Ok(card)
}
