//! Relic Bind — {2}{U} — Enchantment — Aura
//! Oracle: Enchant artifact an opponent controls
//! Oracle: Whenever enchanted artifact becomes tapped, choose one —
//! Oracle: • This Aura deals 1 damage to target player or planeswalker.
//! Oracle: • Target player gains 1 life.
//! Set: 4ED #97 — Fourth Edition | Scryfall ID: 05eee3dd-e17f-4154-8f8d-29c5421cd89d | Oracle ID: 35b9f327-d1f6-47d1-aefa-6f70f407b4bc

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OPPONENT_ARTIFACT: Filter = f!(opponents ARTIFACT);

card!(
    index = index::RELIC_BIND,
    oracle_id = "35b9f327-d1f6-47d1-aefa-6f70f407b4bc",
    scryfall_id = "05eee3dd-e17f-4154-8f8d-29c5421cd89d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Relic Bind",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&OPPONENT_ARTIFACT)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&OPPONENT_ARTIFACT)))
        ),
        modal_triggered!(
            Trigger::BecomesTapped(&Filter::AttachedToBySource),
            &[
                mode!(
                    &[Effect::DealDamage {
                        amount: Amount::Fixed(1),
                        target: TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)
                    }],
                    targets = Some(TargetReq::one(TargetSpec::PlayerOrObject(
                        &Filter::PLANESWALKER
                    )))
                ),
                mode!(
                    &[Effect::GainLifeFor {
                        amount: Amount::Fixed(1),
                        who: PlayerRel::Chosen
                    }],
                    targets = Some(TargetReq::one(TargetSpec::AnyPlayer))
                ),
            ]
        )
    ],
);
