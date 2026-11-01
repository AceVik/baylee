//! Abduction — {2}{U}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When this Aura enters, untap enchanted creature.
//! Oracle: You control enchanted creature.
//! Oracle: When enchanted creature dies, return that card to the battlefield under its owner's control.
//! Set: 6ED #55 — Classic Sixth Edition | Scryfall ID: 63c82bef-50d6-4d25-bc3f-dda2826fc99c | Oracle ID: d0e1904e-1a37-41f6-8582-b9ea794bb886
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ABDUCTION,
    oracle_id = "d0e1904e-1a37-41f6-8582-b9ea794bb886",
    scryfall_id = "63c82bef-50d6-4d25-bc3f-dda2826fc99c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Abduction",
        mana_cost = mana!("{2}{U}{U}"),
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
            Trigger::ETB,
            &[Effect::UntapAll {
                filter: &Filter::AttachedToBySource,
            }],
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::GainControl),
        triggered!(
            Trigger::Dies(&Filter::AttachedToBySource),
            &[Effect::GraveyardToBattlefield {
                target: TargetSpec::EventObject,
                owner_control: true,
                counters: None,
            }],
        ),
    ],
);
