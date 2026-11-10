//! Smoldering Spires — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, target creature can't block this turn.
//! Oracle: {T}: Add {R}.
//! Set: E01 #96 — Archenemy: Nicol Bolas | Scryfall ID: e6741f53-f02e-4f1b-9620-e49ad00e7e1d | Oracle ID: cfa3288d-e521-4a13-bcb3-7950a94e1746
// IMPLEMENTED — enters tapped, {T}: Add {R}, and the enters trigger that
// takes one creature out of the defence.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SMOLDERING_SPIRES,
    oracle_id = "cfa3288d-e521-4a13-bcb3-7950a94e1746",
    scryfall_id = "e6741f53-f02e-4f1b-9620-e49ad00e7e1d",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Smoldering Spires",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        // `Trigger::ETB` is the spelling for "when this enters" about the
        // source itself; the creature is what the ability *targets*, which
        // is a `TargetReq` beside the trigger and not a filter inside it.
        triggered!(
            Trigger::ETB,
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::CANT_BLOCK,
                duration: Duration::UntilEndOfTurn,
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
    ],
);
