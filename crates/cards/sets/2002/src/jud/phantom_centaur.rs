//! Phantom Centaur — {2}{G}{G} — Creature — Centaur Spirit
//! Oracle: Protection from black
//! Oracle: This creature enters with three +1/+1 counters on it.
//! Oracle: If damage would be dealt to this creature, prevent that damage. Remove a +1/+1 counter from this creature.
//! Set: JUD #127 — Judgment | Scryfall ID: c421c2f1-2137-41bd-9a89-74d8a76fb5c5 | Oracle ID: 9bb8c54b-1228-4b7c-8651-52cb5b0f6e72
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTOM_CENTAUR,
    oracle_id = "9bb8c54b-1228-4b7c-8651-52cb5b0f6e72",
    scryfall_id = "c421c2f1-2137-41bd-9a89-74d8a76fb5c5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Phantom Centaur",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CENTAUR, subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
