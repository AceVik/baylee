//! Body Snatcher — {2}{B}{B} — Creature — Phyrexian Minion
//! Oracle: When this creature enters, exile it unless you discard a creature card.
//! Oracle: When this creature dies, exile it and return target creature card from your graveyard to the battlefield.
//! Set: DMR #75 — Dominaria Remastered | Scryfall ID: 2d7d83a8-3bad-40cf-991c-fc7f8e399c42 | Oracle ID: e04688e0-672b-407f-a2c9-fd7292926483
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BODY_SNATCHER,
    oracle_id = "e04688e0-672b-407f-a2c9-fd7292926483",
    scryfall_id = "2d7d83a8-3bad-40cf-991c-fc7f8e399c42",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Body Snatcher",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::MINION],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        triggered!(
            Trigger::ETB,
            &[Effect::PlayerMayPayCostOr {
                player: PlayerRel::You,
                cost: &CostPart::Discard(&Filter::CREATURE),
                effect: &Effect::exile(TargetSpec::ThisObject),
            }]
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[
                Effect::exile(TargetSpec::ThisObject),
                Effect::reanimate(TargetSpec::CardInGraveyard(
                    &Filter::CREATURE,
                    PlayerRel::You,
                )),
            ],
            targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::You,
            ))),
        ),
    ],
);
