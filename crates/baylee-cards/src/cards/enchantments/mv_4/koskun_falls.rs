//! Koskun Falls — {2}{B}{B} — World Enchantment
//! Oracle: At the beginning of your upkeep, sacrifice this enchantment unless you tap an untapped creature you control.
//! Oracle: Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.
//! Set: HML #55 — Homelands | Scryfall ID: 04292a4e-8910-4911-a76d-4f2c3e15da33 | Oracle ID: a9b9e798-debc-4f2c-9770-e16f670082f7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::KOSKUN_FALLS,
    oracle_id = "a9b9e798-debc-4f2c-9770-e16f670082f7",
    scryfall_id = "04292a4e-8910-4911-a76d-4f2c3e15da33",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Koskun Falls",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        supertypes = SupertypeSet::WORLD,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
