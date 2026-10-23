//! Despondency — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets -2/-0.
//! Oracle: When this Aura is put into a graveyard from the battlefield, return it to its owner's hand.
//! Set: USG #129 — Urza's Saga | Scryfall ID: ef400a6a-628a-40d6-80dc-feabe40d6ce1 | Oracle ID: a68ac1ba-f51e-446c-abf0-7a2bd4d65a8c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DESPONDENCY,
    oracle_id = "a68ac1ba-f51e-446c-abf0-7a2bd4d65a8c",
    scryfall_id = "ef400a6a-628a-40d6-80dc-feabe40d6ce1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Despondency",
        mana_cost = mana!("{1}{B}"),
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
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(-2, 0)),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        ),
    ],
);
