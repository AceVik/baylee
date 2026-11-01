//! Hazy Homunculus — {1}{U} — Creature — Homunculus Illusion
//! Oracle: This creature can't be blocked as long as defending player controls an untapped land.
//! Set: PCY #36 — Prophecy | Scryfall ID: f87489f2-82b7-4be6-80ae-3d5955d5ed92 | Oracle ID: 1af26f29-627c-48e5-a19d-deb43a11a846
// HAND-OWNED STUB — adopted with `xtask codegen --adopt-stub`; still unimplemented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HAZY_HOMUNCULUS,
    oracle_id = "1af26f29-627c-48e5-a19d-deb43a11a846",
    scryfall_id = "f87489f2-82b7-4be6-80ae-3d5955d5ed92",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Hazy Homunculus",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HOMUNCULUS, subtypes::creature::ILLUSION],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
