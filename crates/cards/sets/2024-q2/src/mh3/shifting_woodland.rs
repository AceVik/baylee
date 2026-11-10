//! Shifting Woodland — (no cost) — Land
//! Oracle: This land enters tapped unless you control a Forest.
//! Oracle: {T}: Add {G}.
//! Oracle: Delirium — {2}{G}{G}: This land becomes a copy of target permanent card in your graveyard until end of turn. Activate only if there are four or more card types among cards in your graveyard.
//! Set: MH3 #228 — Modern Horizons 3 | Scryfall ID: 059164e1-894d-4586-9800-e60d6fbd6eb6 | Oracle ID: 7c2a4fe5-43e8-4e20-bef2-0278d18afc4b

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

static FOREST_YOU_CONTROL: Filter =
    Filter::And(&[Filter::HasSubtype(land::FOREST), Filter::ControlledByYou]);

card!(
    index = index::SHIFTING_WOODLAND,
    oracle_id = "7c2a4fe5-43e8-4e20-bef2-0278d18afc4b",
    scryfall_id = "059164e1-894d-4586-9800-e60d6fbd6eb6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Shifting Woodland",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&FOREST_YOU_CONTROL)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        activated!(
            cost!("{2}{G}{G}"),
            &[Effect::BecomeCopyOfTarget {
                mods: &[],
                until_eot: true
            }],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::Or(&[
                    Filter::ARTIFACT,
                    Filter::CREATURE,
                    Filter::ENCHANTMENT,
                    Filter::LAND,
                    Filter::PLANESWALKER,
                    Filter::HasType(TypeSet::BATTLE)
                ]),
                PlayerRel::You
            )),
            condition = Some(Condition::GraveyardTypesAtLeast(4))
        ),
    ],
);
