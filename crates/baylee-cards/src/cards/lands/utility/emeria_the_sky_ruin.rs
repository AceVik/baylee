//! Emeria, the Sky Ruin — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: At the beginning of your upkeep, if you control seven or more Plains, you may return target creature card from your graveyard to the battlefield.
//! Oracle: {T}: Add {W}.
//! Set: SOC #368 — Secrets of Strixhaven Commander | Scryfall ID: 90f148e6-1a5e-46fc-9557-40c5c0038069 | Oracle ID: cc999cf2-c99b-4911-8c52-6cc4a99fcc7b
// IMPLEMENTED — enters tapped, {T}: Add {W}, and the upkeep trigger whose
// intervening `if` counts Plains; the printed "you may" is a target with a
// minimum of zero.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;

card!(
    index = index::EMERIA_THE_SKY_RUIN,
    oracle_id = "cc999cf2-c99b-4911-8c52-6cc4a99fcc7b",
    scryfall_id = "90f148e6-1a5e-46fc-9557-40c5c0038069",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Emeria, the Sky Ruin",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::GraveyardToBattlefield {
                target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You),
            }],
            targets = Some(TargetReq::up_to_one(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::You,
            ))),
            condition = Some(Condition::ControlCount(
                &Filter::HasSubtype(land::PLAINS),
                7
            )),
        ),
    ],
);
