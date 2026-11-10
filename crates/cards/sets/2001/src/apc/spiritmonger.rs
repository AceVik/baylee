//! Spiritmonger — {3}{B}{G} — Creature — Beast
//! Oracle: Whenever this creature deals damage to a creature, put a +1/+1 counter on this creature.
//! Oracle: {B}: Regenerate this creature.
//! Oracle: {G}: This creature becomes the color of your choice until end of turn.
//! Set: PLST #DMR-202 — The List | Scryfall ID: 5b1b59cc-57c7-4be2-b9a3-c56703fe65f9 | Oracle ID: 3913c7f2-86b7-4dd4-84a6-9bd4c1c66781
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPIRITMONGER,
    oracle_id = "3913c7f2-86b7-4dd4-84a6-9bd4c1c66781",
    scryfall_id = "5b1b59cc-57c7-4be2-b9a3-c56703fe65f9",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[face!(
        name = "Spiritmonger",
        mana_cost = mana!("{3}{B}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
