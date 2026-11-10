//! Brutal Suppression — {R} — Enchantment
//! Oracle: Activated abilities of nontoken Rebels cost an additional "Sacrifice a land" to activate.
//! Set: PCY #85 — Prophecy | Scryfall ID: 17b3725b-924d-4137-9078-1a28f06c84fa | Oracle ID: b4971aa3-32d1-4bc7-8133-ed67763d2dc6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BRUTAL_SUPPRESSION,
    oracle_id = "b4971aa3-32d1-4bc7-8133-ed67763d2dc6",
    scryfall_id = "17b3725b-924d-4137-9078-1a28f06c84fa",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Brutal Suppression",
        mana_cost = mana!("{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
