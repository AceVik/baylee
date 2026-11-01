//! Laquatus's Champion — {4}{B}{B} — Creature — Nightmare Horror
//! Oracle: When this creature enters, target player loses 6 life.
//! Oracle: When this creature leaves the battlefield, that player gains 6 life.
//! Oracle: {B}: Regenerate this creature.
//! Set: A25 #95 — Masters 25 | Scryfall ID: 0e7ed355-894a-4592-9a64-ef061550d560 | Oracle ID: 693ddb18-00d2-4784-90a0-701a64195034
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LAQUATUS_S_CHAMPION,
    oracle_id = "693ddb18-00d2-4784-90a0-701a64195034",
    scryfall_id = "0e7ed355-894a-4592-9a64-ef061550d560",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Laquatus's Champion",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::HORROR],
        power = Some(6),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
