//! Cave Tiger — {2}{G} — Creature — Cat
//! Oracle: Whenever this creature becomes blocked by a creature, this creature gets +1/+1 until end of turn.
//! Set: USG #241 — Urza's Saga | Scryfall ID: c782eb28-0cdd-4c3d-9d89-8b23c29cddd4 | Oracle ID: d3bac8b8-0b2a-4c67-b087-7a0bf1359468
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CAVE_TIGER,
    oracle_id = "d3bac8b8-0b2a-4c67-b087-7a0bf1359468",
    scryfall_id = "c782eb28-0cdd-4c3d-9d89-8b23c29cddd4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cave Tiger",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CAT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::BecomesBlockedBy(&Filter::CREATURE),
        &[Effect::continuous(
            &Filter::This,
            Modifier::ModifyPT(1, 1),
            Duration::UntilEndOfTurn,
        )],
    )],
);
