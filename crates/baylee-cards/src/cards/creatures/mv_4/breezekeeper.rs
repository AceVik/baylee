//! Breezekeeper — {3}{U} — Creature — Djinn
//! Oracle: Flying
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Set: VIS #27 — Visions | Scryfall ID: beaefa77-6e4a-4724-a443-fa6b45803db5 | Oracle ID: 138439ad-76c6-4d75-8025-e68b94f68fa3
// IMPLEMENTED — flying and phasing.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BREEZEKEEPER,
    oracle_id = "138439ad-76c6-4d75-8025-e68b94f68fa3",
    scryfall_id = "beaefa77-6e4a-4724-a443-fa6b45803db5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::FLYING.union(KeywordSet::PHASING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Breezekeeper",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DJINN],
        power = Some(4),
        toughness = Some(4),
        keywords = KeywordSet::FLYING.union(KeywordSet::PHASING),
    ),],
);
