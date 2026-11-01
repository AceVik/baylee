//! Mercenary Informer — {2}{W} — Creature — Human Rebel Mercenary
//! Oracle: This creature can't be the target of black spells or abilities from black sources.
//! Oracle: {2}{W}: Put target nontoken Mercenary on the bottom of its owner's library.
//! Set: PCY #15 — Prophecy | Scryfall ID: 98ee3f50-09d7-4960-8214-680a7299fa20 | Oracle ID: 245e26ec-39eb-4af8-99af-8cc22edbc261
// HAND-OWNED STUB — adopted with `xtask codegen --adopt-stub`; still unimplemented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MERCENARY_INFORMER,
    oracle_id = "245e26ec-39eb-4af8-99af-8cc22edbc261",
    scryfall_id = "98ee3f50-09d7-4960-8214-680a7299fa20",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Mercenary Informer",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::REBEL,
            subtypes::creature::MERCENARY
        ],
        power = Some(2),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
