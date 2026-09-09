//! Strength of the Harvest // Haven of the Harvest — {2}{G/W} — Enchantment — Aura // Land
//! Set: MH3 #258 — Modern Horizons 3 | Scryfall ID: a7143aa7-b16d-4e63-910c-6ceec55483f3 | Oracle ID: 1a8c996d-ca93-4c17-ace5-66ecd6b99317
//! Face: Strength of the Harvest — {2}{G/W} — Enchantment — Aura
//! Face: Haven of the Harvest —  — Land
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card! {
    index: 1094,
    oracle_id: "1a8c996d-ca93-4c17-ace5-66ecd6b99317",
    scryfall_id: "a7143aa7-b16d-4e63-910c-6ceec55483f3",
    color_identity: ColorSet::from_slice(&[Color::Green, Color::White]),
    faces: &[
    face! {
        name: "Strength of the Harvest",
        mana_cost: baylee_core::mana!("{2}{G/W}"),
        types: TypeSet::ENCHANTMENT,
        subtypes: &[subtypes::enchantment::AURA],
    },
    face! {
        name: "Haven of the Harvest",
        types: TypeSet::LAND,
    },
    ],
}

// TODO(card): implement abilities, see docs/card-dsl.md.
