//! Peace and Quiet — {1}{W} — Instant
//! Oracle: Destroy two target enchantments.
//! Set: ULG #17 — Urza's Legacy | Scryfall ID: 5d73accc-8f19-44d4-8216-c1acdbef3856 | Oracle ID: 0517f736-e6eb-4fdb-8288-e0c8a9a44379
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PEACE_AND_QUIET,
    oracle_id = "0517f736-e6eb-4fdb-8288-e0c8a9a44379",
    scryfall_id = "5d73accc-8f19-44d4-8216-c1acdbef3856",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Peace and Quiet",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[Effect::destroy(TargetSpec::Object(&Filter::ENCHANTMENT))],
        targets = Some(TargetReq::exactly(
            TargetSpec::Object(&Filter::ENCHANTMENT),
            2,
        )),
    )],
);
