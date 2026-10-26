//! Land Grant — {1}{G} — Sorcery
//! Oracle: If you have no land cards in hand, you may reveal your hand rather than pay this spell's mana cost.
//! Oracle: Search your library for a Forest card, reveal that card, put it into your hand, then shuffle.
//! Set: MMQ #255 — Mercadian Masques | Scryfall ID: d6862005-32d1-473e-a28b-5dfc4b7782cd | Oracle ID: e36f4649-934d-4e8a-bd5f-39f164ea6937
// NOT SUPPORTED: conditional alternative cost (reveal hand with no land cards) not supported
// PARTIAL: conditional alternative cost (reveal hand with no land cards) not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const FOREST_CARD: Filter = Filter::HasSubtype(subtypes::land::FOREST);

card!(
    index = index::LAND_GRANT,
    oracle_id = "e36f4649-934d-4e8a-bd5f-39f164ea6937",
    scryfall_id = "d6862005-32d1-473e-a28b-5dfc4b7782cd",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "conditional alternative cost (reveal hand with no land cards) not supported"
    ),
    faces = &[face!(
        name = "Land Grant",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(&[Effect::SearchLibrary {
        filter: &FOREST_CARD,
        finds: &[Find::HAND],
        optional: false,
    }]),],
);
