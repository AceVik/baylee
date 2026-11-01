//! Dromosaur — {2}{R} — Creature — Dinosaur
//! Oracle: Whenever this creature blocks or becomes blocked, it gets +2/-2 until end of turn.
//! Set: USG #182 — Urza's Saga | Scryfall ID: 65de0b43-64df-44dc-850c-25f48d6ab53b | Oracle ID: 4576cc5b-7b3a-41a7-9b84-acadf2d4adf5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DROMOSAUR,
    oracle_id = "4576cc5b-7b3a-41a7-9b84-acadf2d4adf5",
    scryfall_id = "65de0b43-64df-44dc-850c-25f48d6ab53b",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dromosaur",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DINOSAUR],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedByOneOrMore(&Filter::CREATURE),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(2, -2),
            duration: Duration::UntilEndOfTurn,
        }],
    )],
);
