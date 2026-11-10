//! Savaen Elves — {G} — Creature — Elf
//! Oracle: {G}{G}, {T}: Destroy target Aura attached to a land.
//! Set: DRK #84 — The Dark | Scryfall ID: 38fb3014-f631-4a75-92cd-7e626b13a4c3 | Oracle ID: 330e109c-1885-4946-9edc-895408519b3d

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const AURA_ON_LAND: Filter = Filter::And(&[
    Filter::HasSubtype(subtypes::enchantment::AURA),
    Filter::AttachedTo(&Filter::LAND),
]);

card!(
    index = index::SAVAEN_ELVES,
    oracle_id = "330e109c-1885-4946-9edc-895408519b3d",
    scryfall_id = "38fb3014-f631-4a75-92cd-7e626b13a4c3",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Savaen Elves",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELF],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{G}{G}", TapSelf),
        &[Effect::destroy(TargetSpec::Object(&AURA_ON_LAND))],
        target = Some(TargetSpec::Object(&AURA_ON_LAND)),
    )],
);
