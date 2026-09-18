//! Steely Resolve — {1}{G} — Enchantment
//! Oracle: As this enchantment enters, choose a creature type.
//! Oracle: Creatures of the chosen type have shroud. (They can't be the targets of spells or abilities.)
//! Set: ONS #286 — Onslaught | Scryfall ID: b88c530a-abc3-4cc4-8a48-5b76e1504a3c | Oracle ID: 48c127f0-2857-4c36-97bc-1291b6fe4a82
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STEELY_RESOLVE,
    oracle_id = "48c127f0-2857-4c36-97bc-1291b6fe4a82",
    scryfall_id = "b88c530a-abc3-4cc4-8a48-5b76e1504a3c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Steely Resolve",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
