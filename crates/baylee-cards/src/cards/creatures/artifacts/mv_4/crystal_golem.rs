//! Crystal Golem — {4} — Artifact Creature — Golem
//! Oracle: At the beginning of your end step, this creature phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before you untap during your next untap step.)
//! Set: MIR #298 — Mirage | Scryfall ID: 4b1d3280-f3e1-42ea-93e1-dbab7336fb73 | Oracle ID: 82246ef2-4267-4daf-9463-908c2e142e7d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CRYSTAL_GOLEM,
    oracle_id = "82246ef2-4267-4daf-9463-908c2e142e7d",
    scryfall_id = "4b1d3280-f3e1-42ea-93e1-dbab7336fb73",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crystal Golem",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(3),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::End,
            whose: PlayerRel::You,
        },
        &[Effect::PhaseOut { target: None }],
    )],
);
