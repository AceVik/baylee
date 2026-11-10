//! Launch — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has flying.
//! Oracle: When this Aura is put into a graveyard from the battlefield, return it to its owner's hand.
//! Set: USG #82 — Urza's Saga | Scryfall ID: 58f78667-b3ab-44af-89df-9e9332dc5485 | Oracle ID: 9e3b373f-05f5-44df-9568-bd1f8039aa54
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LAUNCH,
    oracle_id = "9e3b373f-05f5-44df-9568-bd1f8039aa54",
    scryfall_id = "58f78667-b3ab-44af-89df-9e9332dc5485",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Launch",
        mana_cost = mana!("{1}{U}"),
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
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::AddKeyword(KeywordSet::FLYING)
        ),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
        ),
    ],
);
