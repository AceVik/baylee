//! Benalish Emissary — {2}{W} — Creature — Human Wizard
//! Oracle: Kicker {1}{G} (You may pay an additional {1}{G} as you cast this spell.)
//! Oracle: When this creature enters, if it was kicked, destroy target land.
//! Set: INV #5 — Invasion | Scryfall ID: 6b82d56e-80d7-4be9-ac22-de3257efc458 | Oracle ID: 99f2e195-5da5-4239-87e0-ed552643e7b5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BENALISH_EMISSARY,
    oracle_id = "99f2e195-5da5-4239-87e0-ed552643e7b5",
    scryfall_id = "6b82d56e-80d7-4be9-ac22-de3257efc458",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Benalish Emissary",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
