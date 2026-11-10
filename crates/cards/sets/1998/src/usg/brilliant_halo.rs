//! Brilliant Halo — {1}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+2.
//! Oracle: When this Aura is put into a graveyard from the battlefield, return it to its owner's hand.
//! Set: VMA #19 — Vintage Masters | Scryfall ID: 6cef6912-eba8-429a-9936-c91c1d42a30a | Oracle ID: 8f68f6d5-1953-4ccd-8af0-7d0a69139556
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BRILLIANT_HALO,
    oracle_id = "8f68f6d5-1953-4ccd-8af0-7d0a69139556",
    scryfall_id = "6cef6912-eba8-429a-9936-c91c1d42a30a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Brilliant Halo",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 2)),
        triggered!(
            Trigger::Dies(&Filter::This),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }]
        ),
    ],
);
