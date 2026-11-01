//! Replenish — {3}{W} — Sorcery
//! Oracle: Return all enchantment cards from your graveyard to the battlefield. (Auras with nothing to enchant remain in your graveyard.)
//! Set: UDS #15 — Urza's Destiny | Scryfall ID: 7fd2fe13-bbc0-42b7-bc42-3b51910ce118 | Oracle ID: 523ae937-5535-490d-96ea-07f331b5e5ad
// NOT SUPPORTED: returning Aura cards attached to an eligible object is not supported, so Aura cards remain in your graveyard.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NON_AURA_ENCHANTMENT: Filter = Filter::And(&[
    Filter::ENCHANTMENT,
    Filter::Not(&Filter::HasSubtype(subtypes::enchantment::AURA)),
]);

card!(
    index = index::REPLENISH,
    oracle_id = "523ae937-5535-490d-96ea-07f331b5e5ad",
    scryfall_id = "7fd2fe13-bbc0-42b7-bc42-3b51910ce118",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "returns all non-Aura enchantment cards from your graveyard to the battlefield; returning Aura cards attached to an eligible object is not supported, so Aura cards remain in your graveyard"
    ),
    faces = &[face!(
        name = "Replenish",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::YourGraveyardToBattlefield {
        filter: &NON_AURA_ENCHANTMENT,
        tapped: false,
    }])],
);
