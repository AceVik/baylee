//! Pit of Offerings — (no cost) — Land — Cave
//! Oracle: This land enters tapped.
//! Oracle: When this land enters, exile up to three target cards from graveyards.
//! Oracle: {T}: Add {C}.
//! Oracle: {T}: Add one mana of any of the exiled cards' colors.
//! Set: LCI #278 — The Lost Caverns of Ixalan | Scryfall ID: bc7d3957-b483-4a1f-a244-293c90032f5e | Oracle ID: 044d2788-6daa-4849-a813-1f577eef9295
// PARTIAL — enters tapped, ETB exile up to three target cards from graveyards, and {T}: Add {C} are built; mana of exiled cards' colors has no ManaSource variant.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PIT_OF_OFFERINGS,
    oracle_id = "044d2788-6daa-4849-a813-1f577eef9295",
    scryfall_id = "bc7d3957-b483-4a1f-a244-293c90032f5e",
    faces = &[face!(
        name = "Pit of Offerings",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::CAVE],
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Partial(
        "producing mana of colors of exiled cards is not expressible (ManaSource has no variant for exiled cards' colors)"
    ),
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::Exile {
                target: TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer),
            }],
            targets = Some(TargetReq::up_to(
                TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer),
                3,
            )),
        ),
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{T}: Add one mana of any of the exiled cards' colors."
    ],
);
