//! Kjeldoran Dead — {B} — Creature — Skeleton
//! Oracle: When this creature enters, sacrifice a creature.
//! Oracle: {B}: Regenerate this creature.
//! Set: ME2 #98 — Masters Edition II | Scryfall ID: 581c6102-99cd-4768-a5d3-3e724490db17 | Oracle ID: 61e00946-7df7-4298-91b2-c656169e6601

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KJELDORAN_DEAD,
    oracle_id = "61e00946-7df7-4298-91b2-c656169e6601",
    scryfall_id = "581c6102-99cd-4768-a5d3-3e724490db17",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Kjeldoran Dead",
        mana_cost = mana!("{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SKELETON],
        power = Some(3),
        toughness = Some(1),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::SacrificeFilter {
                who: PlayerRel::You,
                filter: &Filter::CREATURE,
            }],
        ),
        activated!(cost!("{B}"), &[Effect::regenerate(TargetSpec::ThisObject)],),
    ],
);
