//! Tidal Kraken — {5}{U}{U}{U} — Creature — Kraken
//! Oracle: This creature can't be blocked.
//! Set: 9ED #105 — Ninth Edition | Scryfall ID: 569d1014-e04a-4f8d-a2b2-699b5be08d5d | Oracle ID: 104a8f76-d178-4a33-bb59-e8767da3d982
// IMPLEMENTED — 6/6 Kraken that can't be blocked.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TIDAL_KRAKEN,
    oracle_id = "104a8f76-d178-4a33-bb59-e8767da3d982",
    scryfall_id = "569d1014-e04a-4f8d-a2b2-699b5be08d5d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::UNBLOCKABLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tidal Kraken",
        mana_cost = mana!("{5}{U}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KRAKEN],
        power = Some(6),
        toughness = Some(6),
    ),],
);
