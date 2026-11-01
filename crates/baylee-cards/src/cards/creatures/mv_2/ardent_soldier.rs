//! Ardent Soldier — {1}{W} — Creature — Human Soldier
//! Oracle: Kicker {2} (You may pay an additional {2} as you cast this spell.)
//! Oracle: Vigilance
//! Oracle: If this creature was kicked, it enters with a +1/+1 counter on it.
//! Set: INV #3 — Invasion | Scryfall ID: 39dce974-846f-4365-b0a5-851e38668e7d | Oracle ID: a56d301b-8f4c-4f12-8ee0-34c6b66faee0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ARDENT_SOLDIER,
    oracle_id = "a56d301b-8f4c-4f12-8ee0-34c6b66faee0",
    scryfall_id = "39dce974-846f-4365-b0a5-851e38668e7d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Ardent Soldier",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SOLDIER],
        power = Some(1),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
