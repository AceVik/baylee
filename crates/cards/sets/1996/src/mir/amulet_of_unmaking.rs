//! Amulet of Unmaking — {5} — Artifact
//! Oracle: {5}, {T}, Exile this artifact: Exile target artifact, creature, or land. Activate only as a sorcery.
//! Set: MIR #293 — Mirage | Scryfall ID: fdbd94c8-611c-4b20-99ca-dd2d7661d644 | Oracle ID: 945d5351-ce7d-488c-bc63-269ad82b74cb
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static ARTIFACT_CREATURE_OR_LAND: Filter =
    Filter::Or(&[Filter::ARTIFACT, Filter::CREATURE, Filter::LAND]);

card!(
    index = index::AMULET_OF_UNMAKING,
    oracle_id = "945d5351-ce7d-488c-bc63-269ad82b74cb",
    scryfall_id = "fdbd94c8-611c-4b20-99ca-dd2d7661d644",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Amulet of Unmaking",
        mana_cost = mana!("{5}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{5}", TapSelf, ExileSelf),
        &[Effect::Exile {
            target: TargetSpec::Object(&ARTIFACT_CREATURE_OR_LAND),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &ARTIFACT_CREATURE_OR_LAND,
        ))),
        timing = ActivationTiming::SorcerySpeed,
    )],
);
