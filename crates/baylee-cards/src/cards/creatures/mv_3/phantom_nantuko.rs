//! Phantom Nantuko — {2}{G} — Creature — Insect Spirit
//! Oracle: Trample
//! Oracle: This creature enters with two +1/+1 counters on it.
//! Oracle: If damage would be dealt to this creature, prevent that damage. Remove a +1/+1 counter from this creature.
//! Oracle: {T}: Put a +1/+1 counter on this creature.
//! Set: CMA #133 — Commander Anthology | Scryfall ID: 2fd65594-cf66-4dd4-8d7b-b565756f2f86 | Oracle ID: 0951b529-646c-4dfd-88ad-84ee117ce722
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTOM_NANTUKO,
    oracle_id = "0951b529-646c-4dfd-88ad-84ee117ce722",
    scryfall_id = "2fd65594-cf66-4dd4-8d7b-b565756f2f86",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Phantom Nantuko",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT, subtypes::creature::SPIRIT],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
