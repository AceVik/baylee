//! Carpet of Flowers — {G} — Enchantment
//! Oracle: At the beginning of each of your main phases, if you haven't added mana with this ability this turn, you may add X mana of any one color, where X is the number of Islands target opponent controls.
//! Set: USG #240 — Urza's Saga | Scryfall ID: 93abb48a-85f2-432d-8602-0a1d17fbb409 | Oracle ID: 2ffc6372-f63b-4f32-8dd0-2d7938aeb412
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CARPET_OF_FLOWERS,
    oracle_id = "2ffc6372-f63b-4f32-8dd0-2d7938aeb412",
    scryfall_id = "93abb48a-85f2-432d-8602-0a1d17fbb409",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Carpet of Flowers",
        mana_cost = mana!("{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
