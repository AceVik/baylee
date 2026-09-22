//! Arid Archway — (no cost) — Land — Desert
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, return a land you control to its owner's hand. If another Desert was returned this way, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
//! Oracle: {T}: Add {C}{C}.
//! Set: OTJ #252 — Outlaws of Thunder Junction | Scryfall ID: 3f8c8fa2-12ab-4f6a-9f7a-2bc69e9ba024 | Oracle ID: 3be3d7e6-7860-438a-b8c8-ef154c18c163
// PARTIAL — tapped entry, the mandatory land bounce on arrival, and {C}{C};
// the surveil rider has no conditional in the DSL to hang on.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

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
    coverage = Coverage::Partial(
        "the ETB's second sentence — surveil 1 if another Desert was returned this way"
    ),
    abilities = &[
        // NOT SUPPORTED: "If another Desert was returned this way, surveil 1."
        // The condition is a property of the card that was just returned, and
        // none of `Effect::IfKicked` / `IfEventPowerAtLeast` /
        // `IfCreaturesDiedAtLeast` / `IfNotLostLifeThisTurn` /
        // `IfControlGreatestCmc` / `IfNoCountersOnSelf` asks that question.
        triggered!(
            Trigger::ETB,
            &[Effect::ReturnChosenToHand {
                who: PlayerRel::You,
                filter: &Filter::YOUR_LAND,
            }]
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 2)]),
    ],
);
