//! Phantom Tiger — {2}{G} — Creature — Cat Spirit
//! Oracle: This creature enters with two +1/+1 counters on it.
//! Oracle: If damage would be dealt to this creature, prevent that damage. Remove a +1/+1 counter from this creature.
//! Set: IMA #181 — Iconic Masters | Scryfall ID: 72f8a506-04e4-4f6f-9f81-f31db562f7c6 | Oracle ID: 1755b3d4-0e40-40c4-b913-0960d55d411b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTOM_TIGER,
    oracle_id = "1755b3d4-0e40-40c4-b913-0960d55d411b",
    scryfall_id = "72f8a506-04e4-4f6f-9f81-f31db562f7c6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Phantom Tiger",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CAT, subtypes::creature::SPIRIT],
        power = Some(1),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
