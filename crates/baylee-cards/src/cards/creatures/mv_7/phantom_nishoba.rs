//! Phantom Nishoba — {5}{G}{W} — Creature — Cat Beast Spirit
//! Oracle: Trample
//! Oracle: This creature enters with seven +1/+1 counters on it.
//! Oracle: Whenever this creature deals damage, you gain that much life.
//! Oracle: If damage would be dealt to this creature, prevent that damage. Remove a +1/+1 counter from this creature.
//! Set: DMR #193 — Dominaria Remastered | Scryfall ID: 2281a21b-fc68-46ad-b340-eb13ca0165ec | Oracle ID: e43e06fb-52b7-4f38-8fac-f31973b043f7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHANTOM_NISHOBA,
    oracle_id = "e43e06fb-52b7-4f38-8fac-f31973b043f7",
    scryfall_id = "2281a21b-fc68-46ad-b340-eb13ca0165ec",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Phantom Nishoba",
        mana_cost = mana!("{5}{G}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::CAT,
            subtypes::creature::BEAST,
            subtypes::creature::SPIRIT
        ],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
