//! Lim-Dûl's Cohort — {1}{B}{B} — Creature — Zombie
//! Oracle: Whenever this creature blocks or becomes blocked by a creature, that creature can't be regenerated this turn.
//! Set: ME4 #90 — Masters Edition IV | Scryfall ID: 082cf845-5a24-4f00-bad2-a3d0d07f59e6 | Oracle ID: 1e3b97d2-8fda-4510-9697-f36ca9ca2ab0
// IMPLEMENTED — blocks-or-becomes-blocked-by trigger forbidding regeneration on the combat partner this turn.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LIM_DUL_S_COHORT,
    oracle_id = "1e3b97d2-8fda-4510-9697-f36ca9ca2ab0",
    scryfall_id = "082cf845-5a24-4f00-bad2-a3d0d07f59e6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lim-Dûl's Cohort",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE],
        power = Some(2),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::BlocksOrBecomesBlockedBy(&Filter::CREATURE),
        &[Effect::CantBeRegeneratedThisTurn {
            target: TargetSpec::EventObject,
        }],
    )],
);
