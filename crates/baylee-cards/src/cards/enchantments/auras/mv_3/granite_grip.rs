//! Granite Grip — {2}{R} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+0 for each Mountain you control.
//! Set: 7ED #195 — Seventh Edition | Scryfall ID: 908c8414-5eff-4887-9007-43050058c2d0 | Oracle ID: 19e09405-3294-4f98-a18f-60cf0a8a2e4e
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static MOUNTAIN: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::land::MOUNTAIN),
    Filter::ControlledByYou,
]);

card!(
    index = index::GRANITE_GRIP,
    oracle_id = "19e09405-3294-4f98-a18f-60cf0a8a2e4e",
    scryfall_id = "908c8414-5eff-4887-9007-43050058c2d0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Granite Grip",
        mana_cost = mana!("{2}{R}"),
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
            Modifier::ModifyPTPerCount {
                filter: &MOUNTAIN,
                p: 1,
                t: 0,
            },
        ),
    ],
);
