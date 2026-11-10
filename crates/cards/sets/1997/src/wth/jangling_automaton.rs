//! Jangling Automaton — {3} — Artifact Creature — Construct
//! Oracle: Whenever this creature attacks, untap all creatures defending player controls.
//! Set: WTH #151 — Weatherlight | Scryfall ID: 2e2a427b-9869-4059-aeeb-d9b97b324e4e | Oracle ID: b6f67539-9f76-42b2-96a8-575f27ba928e
// IMPLEMENTED — hand-authored in `baylee-cards`.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static DEFENDERS_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::ControlledByDefendingPlayer]);

card!(
    index = index::JANGLING_AUTOMATON,
    oracle_id = "b6f67539-9f76-42b2-96a8-575f27ba928e",
    scryfall_id = "2e2a427b-9869-4059-aeeb-d9b97b324e4e",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Jangling Automaton",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(3),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Attacks(&Filter::This),
        &[Effect::UntapAll {
            filter: &DEFENDERS_CREATURE
        }]
    ),],
);
