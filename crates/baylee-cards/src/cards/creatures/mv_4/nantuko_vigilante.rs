//! Nantuko Vigilante — {3}{G} — Creature — Insect Druid Mutant
//! Oracle: Morph {1}{G} (You may cast this card face down as a 2/2 creature for {3}. Turn it face up any time for its morph cost.)
//! Oracle: When this creature is turned face up, destroy target artifact or enchantment.
//! Set: MKC #177 — Murders at Karlov Manor Commander | Scryfall ID: ec33d7d5-08f4-4737-bc5f-0be4b595395a | Oracle ID: 28ea78dc-7ad6-4006-aaf8-4b25057ec019
// IMPLEMENTED — morph and mandatory face-up artifact/enchantment destruction.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NANTUKO_VIGILANTE,
    oracle_id = "28ea78dc-7ad6-4006-aaf8-4b25057ec019",
    scryfall_id = "ec33d7d5-08f4-4737-bc5f-0be4b595395a",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Nantuko Vigilante",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::INSECT,
            subtypes::creature::DRUID,
            subtypes::creature::MUTANT
        ],
        power = Some(3),
        toughness = Some(2),
        morph = Some(mana!("{1}{G}")),
    ),],
    abilities = &[triggered!(
        Trigger::TurnedFaceUp,
        &[Effect::destroy(TargetSpec::Object(
            &Filter::ARTIFACT_OR_ENCHANTMENT
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &Filter::ARTIFACT_OR_ENCHANTMENT
        )))
    )],
);
