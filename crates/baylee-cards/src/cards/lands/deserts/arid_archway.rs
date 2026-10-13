//! Arid Archway — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, return a land you control to its owner's hand. If another Desert was returned this way, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Oracle: {T}: Add {C}{C}.
//! Set: OTJ #252 — Outlaws of Thunder Junction | Scryfall ID: 3f8c8fa2-12ab-4f6a-9f7a-2bc69e9ba024 | Oracle ID: 3be3d7e6-7860-438a-b8c8-ef154c18c163
// IMPLEMENTED — enters tapped, returns a land you control to hand on arrival
// (surveil 1 if another Desert was returned this way), and taps for {C}{C}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "another Desert" — a Desert that is not Arid Archway itself.
static ANOTHER_DESERT: Filter =
    Filter::And(&[Filter::HasSubtype(subtypes::land::DESERT), Filter::Another]);

card!(
    index = index::ARID_ARCHWAY,
    oracle_id = "3be3d7e6-7860-438a-b8c8-ef154c18c163",
    scryfall_id = "3f8c8fa2-12ab-4f6a-9f7a-2bc69e9ba024",
    faces = &[face!(
        name = "Arid Archway",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::DESERT],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[
                Effect::ReturnChosenToHand {
                    who: PlayerRel::You,
                    filter: &Filter::YOUR_LAND,
                },
                // "If another Desert was returned this way, surveil 1."
                Effect::IfCondition {
                    condition: Condition::AmountAtLeast(
                        Amount::ThisWay(ThisWay::ReturnedToHand(&ANOTHER_DESERT)),
                        1
                    ),
                    then: &[Effect::Surveil {
                        amount: Amount::Fixed(1)
                    }],
                    otherwise: &[],
                },
            ]
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 2)]),
    ],
);
