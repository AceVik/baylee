//! Dwarven Soldier — {1}{R} — Creature — Dwarf Soldier
//! Oracle: Whenever this creature blocks or becomes blocked by one or more Orcs, this creature gets +0/+2 until end of turn.
//! Set: ME1 #92 — Masters Edition | Scryfall ID: 0dfb3fa6-f4d7-4ed0-b2be-661d59d95800 | Oracle ID: 88f9075c-ea96-4f8d-b0d1-e5e244e824c1
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const ORC: Filter = Filter::HasSubtype(subtypes::creature::ORC);

card!(
    index = index::DWARVEN_SOLDIER,
    oracle_id = "88f9075c-ea96-4f8d-b0d1-e5e244e824c1",
    scryfall_id = "0dfb3fa6-f4d7-4ed0-b2be-661d59d95800",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Dwarven Soldier",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DWARF, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedByOneOrMore(&ORC),
        &[Effect::ModifySource {
            modifier: Modifier::ModifyPT(0, 2),
            duration: Duration::UntilEndOfTurn,
        }],
    ),],
);
