//! Mindstab Thrull — {1}{B}{B} — Creature — Thrull
//! Oracle: Whenever this creature attacks and isn't blocked, you may sacrifice it. If you do, defending player discards three cards.
//! Set: ME1 #76 — Masters Edition | Scryfall ID: 55ef78dd-7336-4f4e-969f-95a7bc604ded | Oracle ID: 43abe1f3-7b92-411a-bf93-e158f949685c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MINDSTAB_THRULL,
    oracle_id = "43abe1f3-7b92-411a-bf93-e158f949685c",
    scryfall_id = "55ef78dd-7336-4f4e-969f-95a7bc604ded",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Mindstab Thrull",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::THRULL],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::AttacksAndIsntBlocked(&Filter::This),
        &[Effect::MayDo {
            effects: &[Effect::SacrificeSelfThen {
                then: &[Effect::DiscardForPlayers {
                    who: PlayerRel::DefendingPlayer,
                    count: 3,
                }],
            }],
        }],
    ),],
);
