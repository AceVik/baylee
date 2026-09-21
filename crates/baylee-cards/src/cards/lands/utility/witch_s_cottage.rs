//! Witch's Cottage — (no cost) — Land — Swamp
//! Oracle: ({T}: Add {B}.)
//! Oracle: This land enters tapped unless you control three or more other Swamps.
//! Oracle: When this land enters untapped, you may put target creature card from your graveyard on top of your library.
//! Set: ELD #249 — Throne of Eldraine | Scryfall ID: b87891cd-b457-4dff-8d18-a7eaf6748fc6 | Oracle ID: 6c8f276e-4e7b-4974-ab02-9356cc0ffb2b
// IMPLEMENTED — {B} off the Swamp type; the enters-tapped replacement asks
// for three other Swamps you control; and the untapped-arrival trigger,
// whose intervening "if" is the source being untapped, asks "you may" and
// puts the targeted creature card on top of its owner's library.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WITCH_S_COTTAGE,
    oracle_id = "6c8f276e-4e7b-4974-ab02-9356cc0ffb2b",
    scryfall_id = "b87891cd-b457-4dff-8d18-a7eaf6748fc6",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Witch's Cottage",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::SWAMP],
        enter_modifiers = &[EnterModifier::TappedUnlessCount {
            filter: &Filter::And(&[
                Filter::HasSubtype(subtypes::land::SWAMP),
                Filter::ControlledByYou,
            ]),
            at_least: 3,
        }],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        triggered!(
            Trigger::ETB,
            &[Effect::MayDo {
                effects: &[Effect::GraveyardToTop {
                    target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
                }],
            }],
            targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::You,
            ))),
            condition = Some(Condition::SourceMatches(&Filter::Untapped)),
        ),
    ],
);
