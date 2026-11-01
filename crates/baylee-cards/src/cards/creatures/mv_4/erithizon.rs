//! Erithizon — {2}{G}{G} — Creature — Beast
//! Oracle: Whenever this creature attacks, put a +1/+1 counter on target creature of defending player's choice.
//! Set: MMQ #244 — Mercadian Masques | Scryfall ID: ec4ea4e2-2102-4b99-bea5-6fc4203f2b26 | Oracle ID: 45645a97-f41d-47fb-b943-293ffc18fc82
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ERITHIZON,
    oracle_id = "45645a97-f41d-47fb-b943-293ffc18fc82",
    scryfall_id = "ec4ea4e2-2102-4b99-bea5-6fc4203f2b26",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Erithizon",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BEAST],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
