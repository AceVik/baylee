//! Pariah — {2}{W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: All damage that would be dealt to you is dealt to enchanted creature instead.
//! Set: USG #28 — Urza's Saga | Scryfall ID: 0447f9e1-792b-4200-9ef3-7cd95c326b88 | Oracle ID: 2c5c8250-1860-42a1-a335-071f54830d37
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PARIAH,
    oracle_id = "2c5c8250-1860-42a1-a335-071f54830d37",
    scryfall_id = "0447f9e1-792b-4200-9ef3-7cd95c326b88",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Pariah",
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
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::RedirectDamageToYou(&Filter::Any),
        ),
    ],
);
