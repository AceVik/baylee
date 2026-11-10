//! Canopy Surge — {1}{G} — Sorcery
//! Oracle: Kicker {2} (You may pay an additional {2} as you cast this spell.)
//! Oracle: Canopy Surge deals 1 damage to each creature with flying and each player. If this spell was kicked, it deals 4 damage to each creature with flying and each player instead.
//! Set: INV #184 — Invasion | Scryfall ID: 2e19d68e-7554-4627-a316-beb1f75fa494 | Oracle ID: a151c45f-0b9e-41af-90b3-fafa12e3bbce
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CANOPY_SURGE,
    oracle_id = "a151c45f-0b9e-41af-90b3-fafa12e3bbce",
    scryfall_id = "2e19d68e-7554-4627-a316-beb1f75fa494",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Canopy Surge",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
