//! Pit Keeper — {1}{B} — Creature — Human Wizard
//! Oracle: When this creature enters, if you have four or more creature cards in your graveyard, you may return target creature card from your graveyard to your hand.
//! Set: TSR #130 — Time Spiral Remastered | Scryfall ID: 5f731d16-d969-40a8-a002-4d40eb8f6bac | Oracle ID: e1c5bfae-07cb-4194-8a86-201503e17085
// IMPLEMENTED — ETB intervening-if creature-graveyard recursion.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PIT_KEEPER,
    oracle_id = "e1c5bfae-07cb-4194-8a86-201503e17085",
    scryfall_id = "5f731d16-d969-40a8-a002-4d40eb8f6bac",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pit Keeper",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::MayDo {
            effects: &[Effect::GraveyardToHand {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            }],
        }],
        targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
            &Filter::CREATURE,
            PlayerRel::You,
        ))),
        condition = Some(Condition::AmountAtLeast(
            Amount::CountOf {
                filter: &Filter::CREATURE,
                zone: ZoneSel::GraveyardYou,
            },
            4,
        )),
    )],
);
