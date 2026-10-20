//! Crown of the Ages — {2} — Artifact
//! Oracle: {4}, {T}: Attach target Aura attached to a creature to another creature.
//! Set: 5ED #360 — Fifth Edition | Scryfall ID: 77b549e5-c25d-4688-b362-faab109ba092 | Oracle ID: 0b772ef2-b760-400b-8e5b-f4b98e6c935d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const AURA_ATTACHED_TO_CREATURE: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::AttachedTo(&Filter::CREATURE),
]);

card!(
    index = index::CROWN_OF_THE_AGES,
    oracle_id = "0b772ef2-b760-400b-8e5b-f4b98e6c935d",
    scryfall_id = "77b549e5-c25d-4688-b362-faab109ba092",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Crown of the Ages",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{4}", TapSelf),
        &[Effect::MoveTargetAuraToAnother {
            types: TypeSet::CREATURE,
        }],
        target = Some(TargetSpec::Object(&AURA_ATTACHED_TO_CREATURE)),
    )],
);
