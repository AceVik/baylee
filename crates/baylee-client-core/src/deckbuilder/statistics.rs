//! Composition of the main deck; sideboards never alter opening-hand odds.
use super::{DeckBuilder, Zone};
use std::collections::BTreeSet;

/// Statistics computed from card quantities, independent of chosen printings.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Statistics {
    /// Distinct rules identities in the main deck.
    pub unique: usize,
    /// Quantity-weighted nonland mana value; absent for all-land decks.
    pub average_mana: Option<f64>,
    /// Fraction of main-deck cards that are lands.
    pub land_share: Option<f64>,
    /// Probability of at least one land in seven cards without replacement,
    /// before mulligans; absent for decks smaller than seven.
    pub opening_land: Option<f64>,
}

impl DeckBuilder {
    /// The current composition, excluding the sideboard and commander zone.
    #[must_use]
    pub fn statistics(&self) -> Statistics {
        let counts = self.counts();
        let mut slots = BTreeSet::new();
        let mut mana = 0.0;
        let mut nonlands = 0_u32;
        for entry in self.entries(Zone::Main) {
            let Some(card) = self.card(entry.slot) else {
                continue;
            };
            slots.insert(card.index);
            if !card.is("Land") {
                mana += f64::from(entry.count) * f64::from(card.cmc);
                nonlands += u32::from(entry.count);
            }
        }
        Statistics {
            unique: slots.len(),
            average_mana: (nonlands > 0).then(|| mana / f64::from(nonlands)),
            land_share: (counts.main > 0).then(|| f64::from(counts.lands) / f64::from(counts.main)),
            opening_land: (counts.main >= 7).then(|| {
                1.0 - (0..7)
                    .map(|draw| {
                        f64::from((counts.main - counts.lands).saturating_sub(draw))
                            / f64::from(counts.main - draw)
                    })
                    .product::<f64>()
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deckbuilder::{Coverage, PoolCard};
    #[test]
    fn counts_copies_excludes_sideboard_and_uses_sampling_without_replacement() {
        let mut deck = DeckBuilder::new();
        deck.set_pool(
            vec![
                PoolCard {
                    index: 1,
                    english_name: "Forest".into(),
                    name: "Forest".into(),
                    kinds: vec!["Land".into()],
                    basic_land: true,
                    coverage: Coverage::Implemented,
                    ..PoolCard::default()
                },
                PoolCard {
                    index: 2,
                    english_name: "Elf".into(),
                    name: "Elf".into(),
                    kinds: vec!["Creature".into()],
                    cmc: 3,
                    coverage: Coverage::Implemented,
                    ..PoolCard::default()
                },
            ],
            true,
        );
        deck.load(
            "d",
            "test",
            &["4 Forest".into(), "4 Elf".into()],
            &["15 Forest".into()],
            &[],
        );
        let stats = deck.statistics();
        assert_eq!(stats.unique, 2);
        assert_eq!(stats.average_mana, Some(3.0));
        assert_eq!(stats.land_share, Some(0.5));
        assert_eq!(stats.opening_land, Some(1.0));
        deck.load("d", "test", &["60 Forest".into()], &[], &[]);
        assert_eq!(deck.statistics().average_mana, None);
        deck.start_new();
        assert_eq!(deck.statistics(), Statistics::default());
    }
}
