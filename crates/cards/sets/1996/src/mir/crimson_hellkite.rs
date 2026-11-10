//! Crimson Hellkite — {6}{R}{R}{R} — Creature — Dragon
//! Oracle: Flying
//! Oracle: {X}, {T}: This creature deals X damage to target creature. Spend only red mana on X.
//! Set: 7ED #178 — Seventh Edition | Scryfall ID: 5451e44f-8dd2-48c6-ba67-5c62a04819ef | Oracle ID: 37a1f340-10d5-407e-bf2b-cbbbb6cab985
// NOT SUPPORTED: "{X}, {T}: deals X damage to target creature. Spend only red mana on X" — an activated ability's X cannot be limited to red mana, and without the limit any colour would pay X (stronger than printed), so the ability is left off.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRIMSON_HELLKITE,
    oracle_id = "37a1f340-10d5-407e-bf2b-cbbbb6cab985",
    scryfall_id = "5451e44f-8dd2-48c6-ba67-5c62a04819ef",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "has flying; the {X}, {T} damage ability is left off because \"spend only red mana on X\" cannot be said for an activated ability, and with any colour paying X it would be stronger than printed",
    ),
    faces = &[face!(
        name = "Crimson Hellkite",
        mana_cost = mana!("{6}{R}{R}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(6),
        toughness = Some(6),
        keywords = KeywordSet::FLYING,
    ),],
);
