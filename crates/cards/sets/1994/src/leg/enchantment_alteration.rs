//! Enchantment Alteration — {U} — Instant
//! Oracle: Attach target Aura attached to a creature or land to another permanent of that type.
//! Set: USG #72 — Urza's Saga | Scryfall ID: 254aa8d0-f0f5-4fb2-a6ba-07453d71e229 | Oracle ID: c80935bf-c17f-4940-ad9f-3f4e3e8f71bc
// IMPLEMENTED — the targeted Aura moves to another permanent of the type it was on; the new host is chosen on resolution.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Aura attached to a creature or land": the only target; the new host is not
/// targeted (2005-08-01 ruling).
const AURA_ON_CREATURE_OR_LAND: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::AttachedTo(&Filter::Or(&[Filter::CREATURE, Filter::LAND])),
]);

card!(
    index = index::ENCHANTMENT_ALTERATION,
    oracle_id = "c80935bf-c17f-4940-ad9f-3f4e3e8f71bc",
    scryfall_id = "254aa8d0-f0f5-4fb2-a6ba-07453d71e229",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Enchantment Alteration",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::MoveTargetAuraToAnother {
            types: TypeSet::CREATURE.union(TypeSet::LAND),
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &AURA_ON_CREATURE_OR_LAND
        )))
    )],
);
