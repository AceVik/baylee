//! Lead Golem — {5} — Artifact Creature — Golem
//! Oracle: Whenever this creature attacks, it doesn't untap during its controller's next untap step.
//! Set: 6ED #296 — Classic Sixth Edition | Scryfall ID: b2ba3857-9bd7-434d-b77f-d697776f33e4 | Oracle ID: 5810c1e2-5a36-4a87-be4e-890ba1627b46
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LEAD_GOLEM,
    oracle_id = "5810c1e2-5a36-4a87-be4e-890ba1627b46",
    scryfall_id = "b2ba3857-9bd7-434d-b77f-d697776f33e4",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Lead Golem",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GOLEM],
        power = Some(3),
        toughness = Some(5),
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::SuppressUntap {
            target: TargetSpec::ThisObject,
            steps: 1,
        }],
    )],
);
