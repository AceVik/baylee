//! Badgermole Cub — {1}{G} — Creature — Badger Mole
//! Oracle: When this creature enters, earthbend 1. (Target land you control becomes a 0/0 creature with haste that's still a land. Put a +1/+1 counter on it. When it dies or is exiled, return it to the battlefield tapped.)
//! Oracle: Whenever you tap a creature for mana, add an additional {G}.
//! Set: TLA #167 — Avatar: The Last Airbender | Scryfall ID: 340c5799-4964-44dd-8c48-8f3f3aba5211 | Oracle ID: 2b0afb89-0944-4861-b9c3-e909e2ac215e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BADGERMOLE_CUB,
    oracle_id = "2b0afb89-0944-4861-b9c3-e909e2ac215e",
    scryfall_id = "340c5799-4964-44dd-8c48-8f3f3aba5211",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Badgermole Cub",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BADGER, subtypes::creature::MOLE],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
