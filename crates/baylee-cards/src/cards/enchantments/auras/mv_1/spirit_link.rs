//! Spirit Link — {W} — Enchantment — Aura
//! Oracle: Enchant creature (Target a creature as you cast this. This card enters attached to that creature.)
//! Oracle: Whenever enchanted creature deals damage, you gain that much life.
//! Set: DMR #29 — Dominaria Remastered | Scryfall ID: fc046a97-4699-438d-8da4-022560a18564 | Oracle ID: c77ff526-c0a8-45c7-9730-2e306a0d01b8

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPIRIT_LINK,
    oracle_id = "c77ff526-c0a8-45c7-9730-2e306a0d01b8",
    scryfall_id = "fc046a97-4699-438d-8da4-022560a18564",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spirit Link",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::DealsDamage(&Filter::AttachedToBySource),
            &[Effect::GainLife {
                amount: Amount::EventAmount
            }]
        ),
    ],
);
