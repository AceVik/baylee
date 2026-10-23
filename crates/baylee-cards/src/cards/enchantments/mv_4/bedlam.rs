//! Bedlam — {2}{R}{R} — Enchantment
//! Oracle: Creatures can't block.
//! Set: 7ED #174 — Seventh Edition | Scryfall ID: 2788ec1a-930e-4a19-ad69-ea456c1390fd | Oracle ID: c28d4caf-9b7c-40b1-9cbe-275ca9dae06a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BEDLAM,
    oracle_id = "c28d4caf-9b7c-40b1-9cbe-275ca9dae06a",
    scryfall_id = "2788ec1a-930e-4a19-ad69-ea456c1390fd",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bedlam",
        mana_cost = mana!("{2}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[static_ability!(
        Filter::CREATURE,
        Modifier::CantBlock(&Filter::CREATURE)
    )],
);
