//! Jackal Pup — {R} — Creature — Jackal
//! Oracle: Whenever this creature is dealt damage, it deals that much damage to you.
//! Set: A25 #139 — Masters 25 | Scryfall ID: 9d8743b2-30e3-4d15-89fc-72974747aec5 | Oracle ID: 993e41e6-0643-4a1f-8ada-b7a4aae486d9
// IMPLEMENTED — damage dealt to this creature is repeated to you, including lethal damage.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JACKAL_PUP,
    oracle_id = "993e41e6-0643-4a1f-8ada-b7a4aae486d9",
    scryfall_id = "9d8743b2-30e3-4d15-89fc-72974747aec5",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jackal Pup",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::JACKAL],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::DealtDamage(&Filter::This),
        &[Effect::DealDamage {
            target: TargetSpec::Player(PlayerRel::You),
            amount: Amount::EventAmount,
        }],
    )],
);
