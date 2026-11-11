//! Bloodthirsty Conqueror — {3}{B}{B} — Creature — Vampire Knight
//! Oracle: Flying, deathtouch
//! Oracle: Whenever an opponent loses life, you gain that much life. (Damage causes loss of life.)
//! Set: FDN #58 — Foundations | Scryfall ID: ce860ed4-a5bd-4347-9eab-dd716ea84db1 | Oracle ID: fd505c02-c59e-476d-8e88-35da862ddc23
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLOODTHIRSTY_CONQUEROR,
    oracle_id = "fd505c02-c59e-476d-8e88-35da862ddc23",
    scryfall_id = "ce860ed4-a5bd-4347-9eab-dd716ea84db1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Bloodthirsty Conqueror",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::VAMPIRE, subtypes::creature::KNIGHT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
