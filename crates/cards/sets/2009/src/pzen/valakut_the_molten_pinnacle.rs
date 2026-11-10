//! Valakut, the Molten Pinnacle — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: Whenever a Mountain you control enters, if you control at least five other Mountains, you may have this land deal 3 damage to any target.
//! Oracle: {T}: Add {R}.
//! Set: ZEN #228 — Zendikar | Scryfall ID: 37bce60d-2cb0-4772-9f5c-122a7ed426a0 | Oracle ID: 1bc44216-4e06-4f66-89b7-5c327004604e

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::land;
static MOUNTAINS_YOU_CONTROL: Filter = f!(your Filter::HasSubtype(land::MOUNTAIN));

card!(
    index = index::VALAKUT_THE_MOLTEN_PINNACLE,
    oracle_id = "1bc44216-4e06-4f66-89b7-5c327004604e",
    scryfall_id = "37bce60d-2cb0-4772-9f5c-122a7ed426a0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Valakut, the Molten Pinnacle",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        triggered!(
            Trigger::EntersBattlefield(&MOUNTAINS_YOU_CONTROL),
            &[Effect::MayDo {
                effects: &[Effect::DealDamage {
                    amount: Amount::Fixed(3),
                    target: TargetSpec::AnyTarget
                }]
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
            condition = Some(Condition::ControlCountExceptEvent(
                &MOUNTAINS_YOU_CONTROL,
                5
            )),
        ),
        mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
    ],
);
