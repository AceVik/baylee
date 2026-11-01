//! Giant Caterpillar — {3}{G} — Creature — Insect
//! Oracle: {G}, Sacrifice this creature: Create a 1/1 green Insect creature token with flying named Butterfly at the beginning of the next end step.
//! Set: MMQ #249 — Mercadian Masques | Scryfall ID: bdc5eb8a-4531-4408-90fe-9b352d71a052 | Oracle ID: 43a515f2-b53a-4bb4-bb6e-0d79c9162359
// IMPLEMENTED

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GIANT_CATERPILLAR,
    oracle_id = "43a515f2-b53a-4bb4-bb6e-0d79c9162359",
    scryfall_id = "bdc5eb8a-4531-4408-90fe-9b352d71a052",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Giant Caterpillar",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INSECT],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[activated!(
        cost!("{G}", SacrificeSelf),
        &[Effect::AtNextEndStep {
            effects: &[Effect::CreateToken {
                token: &generated_tokens::BUTTERFLY_1_1_GREEN_FLYING,
            }],
        }],
    )],
);
