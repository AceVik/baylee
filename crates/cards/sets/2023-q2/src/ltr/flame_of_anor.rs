//! Flame of Anor — {1}{U}{R} — Instant
//! Oracle: Choose one. If you control a Wizard as you cast this spell, you may choose two instead.
//! Oracle: • Target player draws two cards.
//! Oracle: • Destroy target artifact.
//! Oracle: • Flame of Anor deals 5 damage to target creature.
//! Set: LTR #203 — The Lord of the Rings: Tales of Middle-earth | Scryfall ID: 04779a7e-b453-48b9-b392-6d6fd0b8d283 | Oracle ID: ecd49d85-9c8c-4cc9-9a83-072ecd433677
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLAME_OF_ANOR,
    oracle_id = "ecd49d85-9c8c-4cc9-9a83-072ecd433677",
    scryfall_id = "04779a7e-b453-48b9-b392-6d6fd0b8d283",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Flame of Anor",
        mana_cost = mana!("{1}{U}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
