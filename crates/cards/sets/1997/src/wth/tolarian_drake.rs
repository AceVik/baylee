//! Tolarian Drake — {2}{U} — Creature — Drake
//! Oracle: Flying
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Set: WTH #55 — Weatherlight | Scryfall ID: e04bba8a-48ee-4981-adc2-4f82c0f2c1bd | Oracle ID: d999f2a3-5cb4-4f12-a79a-c5db0a300620
// IMPLEMENTED — Flying and phasing.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TOLARIAN_DRAKE,
    oracle_id = "d999f2a3-5cb4-4f12-a79a-c5db0a300620",
    scryfall_id = "e04bba8a-48ee-4981-adc2-4f82c0f2c1bd",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING.union(KeywordSet::PHASING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tolarian Drake",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAKE],
        power = Some(2),
        toughness = Some(4),
        keywords = KeywordSet::FLYING.union(KeywordSet::PHASING),
    ),],
);
