//! Cessation — {2}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't attack.
//! Oracle: When this Aura is put into a graveyard from the battlefield, return it to its owner's hand.
//! Set: ULG #4 — Urza's Legacy | Scryfall ID: 3a113f0c-8249-427b-979b-10898ec66a3a | Oracle ID: a8b3a35e-0172-4f7b-b50f-571e59d7ca5f
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CESSATION,
    oracle_id = "a8b3a35e-0172-4f7b-b50f-571e59d7ca5f",
    scryfall_id = "3a113f0c-8249-427b-979b-10898ec66a3a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cessation",
        mana_cost = mana!("{2}{W}"),
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
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::CANT_ATTACK),
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        ),
    ],
);
