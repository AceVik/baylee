//! Servant of Volrath — {2}{B} — Creature — Minion
//! Oracle: When this creature leaves the battlefield, sacrifice a creature.
//! Set: TMP #156 — Tempest | Scryfall ID: 691afabb-f266-45fd-b5a3-577be4f10f86 | Oracle ID: 4e389b61-7b31-42ba-a19b-2502f63fd34a

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SERVANT_OF_VOLRATH,
    oracle_id = "4e389b61-7b31-42ba-a19b-2502f63fd34a",
    scryfall_id = "691afabb-f266-45fd-b5a3-577be4f10f86",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Servant of Volrath",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MINION],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::LeavesBattlefield(&Filter::This),
        &[Effect::SacrificeFilter {
            who: PlayerRel::You,
            filter: &Filter::CREATURE,
        }],
    )],
);
