//! Consecrate Land — {W} — Enchantment — Aura
//! Oracle: Enchant land
//! Oracle: Enchanted land has indestructible and can't be enchanted by other Auras.
//! Set: TSB #4 — Time Spiral Timeshifted | Scryfall ID: ded79afb-2a65-49e8-81c3-757e5d4c2203 | Oracle ID: 4627691c-4ed4-4add-9cc3-2e019be2f9fd
// IMPLEMENTED — enchanted land has indestructible and permits only this Aura.
// Tests: engine/card_tests/enchantments/consecrate_land.rs.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CONSECRATE_LAND,
    oracle_id = "4627691c-4ed4-4add-9cc3-2e019be2f9fd",
    scryfall_id = "ded79afb-2a65-49e8-81c3-757e5d4c2203",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Consecrate Land",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::LAND)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND)))
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE)
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::CantBeEnchantedExceptSource
        ),
    ],
);
