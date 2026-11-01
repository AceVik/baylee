//! Clockwork Gnomes — {4} — Artifact Creature — Gnome
//! Oracle: {3}, {T}: Regenerate target artifact creature.
//! Set: ME4 #191 — Masters Edition IV | Scryfall ID: f74a8952-fad0-40c3-88eb-3cc05acaf590 | Oracle ID: 21d269f2-c11c-4094-b526-500f0785f483
// IMPLEMENTED — {3}, {T}: Regenerate target artifact creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ARTIFACT_CREATURE: Filter = Filter::And(&[Filter::ARTIFACT, Filter::CREATURE]);

card!(
    index = index::CLOCKWORK_GNOMES,
    oracle_id = "21d269f2-c11c-4094-b526-500f0785f483",
    scryfall_id = "f74a8952-fad0-40c3-88eb-3cc05acaf590",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Clockwork Gnomes",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::GNOME],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[Effect::Regenerate {
            target: TargetSpec::Object(&ARTIFACT_CREATURE),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&ARTIFACT_CREATURE)))
    )],
);
