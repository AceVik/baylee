//! Lightning Greaves — {2} — Artifact — Equipment
//! Oracle: Equipped creature has haste and shroud. (It can't be the target of spells or abilities.)
//! Oracle: Equip {0}
//! Set: MSC #202 — Marvel Super Heroes Commander | Scryfall ID: b61634ae-05be-4b56-8ebb-9d4ade902e42 | Oracle ID: ca204b66-8d0c-431a-8d34-282f7c2d17da
// IMPLEMENTED — one static granting both keywords, plus equip.
//
// Equip {0} is a cost of no mana and no parts at all: equipping does not tap
// the Equipment, and it stays sorcery-speed (CR 702.6b). What makes Lightning
// Greaves what it is, is that the cost is nothing — not that it is faster
// than every other Equipment.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::artifact;

/// Equip targets "target creature you control" (CR 702.6a).
static CREATURE_YOU_CONTROL: Filter = Filter::And(&[Filter::CREATURE, Filter::ControlledByYou]);

card! {
    index: 1351,
    oracle_id: "ca204b66-8d0c-431a-8d34-282f7c2d17da",
    scryfall_id: "b61634ae-05be-4b56-8ebb-9d4ade902e42",
    coverage: Coverage::Implemented,
    faces: &[
    face! {
        name: "Lightning Greaves",
        mana_cost: baylee_core::mana!("{2}"),
        types: TypeSet::ARTIFACT,
        subtypes: &[artifact::EQUIPMENT],
    },
    ],
    abilities: &[
        AbilityDef::Static(StaticAbility {
            layer: Layer::Ability,
            filter: Filter::AttachedToBySource,
            modifier: Modifier::AddKeyword(KeywordSet::HASTE.union(KeywordSet::SHROUD)),
            cross_zone: false,
        }),
        activated!(
            Cost {
                mana: ManaCost::ZERO,
                parts: &[],
            },
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&CREATURE_YOU_CONTROL),
            }],
            target: Some(TargetSpec::Object(&CREATURE_YOU_CONTROL)),
            timing: ActivationTiming::SorcerySpeed
        ),
    ],
}
