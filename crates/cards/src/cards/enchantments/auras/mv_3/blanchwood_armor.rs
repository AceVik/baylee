//! Blanchwood Armor — {2}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+1 for each Forest you control.
//! Set: FDN #213 — Foundations | Scryfall ID: 1fd7ec1a-dafa-42ca-bc25-f6848fb03f60 | Oracle ID: 80ea56ad-e741-4a85-b4e8-ce62e7d593d5
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLANCHWOOD_ARMOR,
    oracle_id = "80ea56ad-e741-4a85-b4e8-ce62e7d593d5",
    scryfall_id = "1fd7ec1a-dafa-42ca-bc25-f6848fb03f60",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Blanchwood Armor",
        mana_cost = mana!("{2}{G}"),
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
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ModifyPTByCount(PtCount::YouControl(&Filter::HasSubtype(
                subtypes::land::FOREST
            )))
        ),
    ],
);
