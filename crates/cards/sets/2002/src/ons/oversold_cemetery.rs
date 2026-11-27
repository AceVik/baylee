//! Oversold Cemetery — {1}{B} — Enchantment
//! Oracle: At the beginning of your upkeep, if you have four or more creature cards in your graveyard, you may return target creature card from your graveyard to your hand.
//! Set: DMR #96 — Dominaria Remastered | Scryfall ID: 06b7312a-c775-4fa8-ba62-c84ff38459e8 | Oracle ID: ed4cd3a1-688b-4c05-948d-39d3336e00c0
// IMPLEMENTED — upkeep intervening-if creature-graveyard recursion.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OVERSOLD_CEMETERY,
    oracle_id = "ed4cd3a1-688b-4c05-948d-39d3336e00c0",
    scryfall_id = "06b7312a-c775-4fa8-ba62-c84ff38459e8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Oversold Cemetery",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::Upkeep,
            whose: PlayerRel::You,
        },
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
