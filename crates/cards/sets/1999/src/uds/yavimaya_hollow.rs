//! Yavimaya Hollow — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {G}, {T}: Regenerate target creature.
//! Set: VMA #325 — Vintage Masters | Scryfall ID: d9fdbc02-7ab7-4f77-8a89-5a9e01eb32f5 | Oracle ID: 53d6113d-acdb-4754-9641-f7991a96c7b9
// IMPLEMENTED — {T}: Add {C}, and the {G}, {T} regeneration shield.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::YAVIMAYA_HOLLOW,
    oracle_id = "53d6113d-acdb-4754-9641-f7991a96c7b9",
    scryfall_id = "d9fdbc02-7ab7-4f77-8a89-5a9e01eb32f5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Yavimaya Hollow",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{G}", TapSelf),
            &[Effect::regenerate(TargetSpec::Object(&Filter::CREATURE))],
            target = Some(TargetSpec::Object(&Filter::CREATURE))
        ),
    ],
);
