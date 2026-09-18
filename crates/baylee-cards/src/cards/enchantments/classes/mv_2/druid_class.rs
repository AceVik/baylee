//! Druid Class — {1}{G} — Enchantment — Class
//! Oracle: (Gain the next level as a sorcery to add its ability.)
//! Oracle: Landfall — Whenever a land you control enters, you gain 1 life.
//! Oracle: {2}{G}: Level 2
//! Oracle: You may play an additional land on each of your turns.
//! Oracle: {4}{G}: Level 3
//! Oracle: When this Class becomes level 3, target land you control becomes a creature with haste and "This creature's power and toughness are each equal to the number of lands you control." It's still a land.
//! Set: AFR #180 — Adventures in the Forgotten Realms | Scryfall ID: 09278e95-eaae-4cd4-a0d8-a2d15b0abb58 | Oracle ID: dcbcbf42-4654-487a-acad-21f2606d229b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DRUID_CLASS,
    oracle_id = "dcbcbf42-4654-487a-acad-21f2606d229b",
    scryfall_id = "09278e95-eaae-4cd4-a0d8-a2d15b0abb58",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Druid Class",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::CLASS],
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
