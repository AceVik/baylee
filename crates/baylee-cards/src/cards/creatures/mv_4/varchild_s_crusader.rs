//! Varchild's Crusader — {3}{R} — Creature — Human Knight
//! Oracle: {0}: This creature can't be blocked this turn except by Walls. Sacrifice this creature at the beginning of the next end step.
//! Set: ME2 #152 — Masters Edition II | Scryfall ID: e5232322-f4e2-49cf-80ed-6876ea485b37 | Oracle ID: 45351d9d-453f-4245-84c0-053eb8aca345
// IMPLEMENTED — {0}: can't be blocked this turn except by Walls; sacrifice at the beginning of the next end step.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static NON_WALL_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::HasSubtype(subtypes::creature::WALL)),
]);

card!(
    index = index::VARCHILD_S_CRUSADER,
    oracle_id = "45351d9d-453f-4245-84c0-053eb8aca345",
    scryfall_id = "e5232322-f4e2-49cf-80ed-6876ea485b37",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Varchild's Crusader",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::KNIGHT],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{0}"),
        &[
            Effect::ModifySource {
                modifier: Modifier::CantBeBlockedBy(&NON_WALL_CREATURE),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::AtNextEndStep {
                effects: &[Effect::SacrificeSelf],
            },
        ],
    )],
);
