//! Slow Motion — {2}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, that player sacrifices that creature unless they pay {2}.
//! Oracle: When this Aura is put into a graveyard from the battlefield, return it to its owner's hand.
//! Set: ULG #42 — Urza's Legacy | Scryfall ID: 2860a20d-e1bf-4e46-8c07-a858f616d5a5 | Oracle ID: 151388a4-a24b-4089-a323-e7c1d5e6490a
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SLOW_MOTION,
    oracle_id = "151388a4-a24b-4089-a323-e7c1d5e6490a",
    scryfall_id = "2860a20d-e1bf-4e46-8c07-a858f616d5a5",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Slow Motion",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached,
            },
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::ControllerOfAttached,
                mana: Amount::Fixed(2),
                effect: &Effect::SacrificeAll {
                    filter: &Filter::AttachedToBySource,
                },
            }],
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        ),
    ],
);
