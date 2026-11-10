//! Amonkhet Raceway — (no cost) — Land
//! Oracle: Start your engines! (If you have no speed, it starts at 1. It increases once on each of your turns when an opponent loses life. Max speed is 4.)
//! Oracle: {T}: Add {C}.
//! Oracle: Max speed — {T}: Target creature gains haste until end of turn.
//! Set: DFT #248 — Aetherdrift | Scryfall ID: 4f312807-ea2a-4385-8774-4e23b4a5d4a6 | Oracle ID: fe174586-d36b-40f6-babd-1f98e76eec22
// IMPLEMENTED — persistent speed starts through the keyword; max speed unlocks the second activation.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AMONKHET_RACEWAY,
    oracle_id = "fe174586-d36b-40f6-babd-1f98e76eec22",
    scryfall_id = "4f312807-ea2a-4385-8774-4e23b4a5d4a6",
    faces = &[face!(name = "Amonkhet Raceway", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            Cost::TAP,
            &[Effect::PumpTarget {
                power: Amount::Fixed(0),
                toughness: Amount::Fixed(0),
                keywords: KeywordSet::HASTE,
                duration: Duration::UntilEndOfTurn
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
            condition = Some(Condition::MaxSpeed)
        ),
        AbilityDef::StartYourEngines,
    ],
);
