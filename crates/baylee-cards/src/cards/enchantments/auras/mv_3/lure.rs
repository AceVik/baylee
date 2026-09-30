//! Lure — {1}{G}{G} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: All creatures able to block enchanted creature do so.
//! Set: IMA #175 — Iconic Masters | Scryfall ID: 72c8336d-54cf-45af-a9ef-a1428facf91b | Oracle ID: 7a7425ba-4478-4bc4-855f-abf947ea4fa2
// PARTIAL — forced blocks are not in the engine; the Aura attaches and does
// nothing.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LURE,
    oracle_id = "7a7425ba-4478-4bc4-855f-abf947ea4fa2",
    scryfall_id = "72c8336d-54cf-45af-a9ef-a1428facf91b",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "forced blocks are not in the engine; the Aura attaches and does nothing"
    ),
    faces = &[face!(
        name = "Lure",
        mana_cost = mana!("{1}{G}{G}"),
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
        // NOT SUPPORTED: All creatures able to block enchanted creature do so.
    ],
);
