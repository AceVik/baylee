//! Transmogrifying Licid — {3} — Artifact Creature — Licid
//! Oracle: {1}, {T}: This creature loses this ability and becomes an Aura enchantment with enchant creature. Attach it to target creature. You may pay {1} to end this effect.
//! Oracle: Enchanted creature gets +1/+1 and is an artifact in addition to its other types.
//! Set: EXO #141 — Exodus | Scryfall ID: 1a115563-81da-42f6-95c4-22ae7bb51a0f | Oracle ID: 4f484927-0e8e-493d-93bc-ed190965dad9
// PARTIAL — Licid creature-to-Aura reconfiguration and ending the effect are not supported; plays as a 2/2 artifact creature whose Aura static abilities give an attached creature +1/+1 and make it an artifact in addition to its other types

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Licid creature-to-Aura reconfiguration and ending the effect are not supported; plays as a 2/2 artifact creature whose Aura static abilities give an attached creature +1/+1 and make it an artifact in addition to its other types
card!(
    index = index::TRANSMOGRIFYING_LICID,
    oracle_id = "4f484927-0e8e-493d-93bc-ed190965dad9",
    scryfall_id = "1a115563-81da-42f6-95c4-22ae7bb51a0f",
    coverage = Coverage::Partial(
        "Licid creature-to-Aura reconfiguration and ending the effect are not supported; plays as a 2/2 artifact creature whose Aura static abilities give an attached creature +1/+1 and make it an artifact in addition to its other types",
    ),
    faces = &[face!(
        name = "Transmogrifying Licid",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::LICID],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 1)),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddType(TypeSet::ARTIFACT),
        ),
    ],
);
