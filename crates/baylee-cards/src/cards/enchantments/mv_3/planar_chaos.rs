//! Planar Chaos — {2}{R} — Enchantment
//! Oracle: At the beginning of your upkeep, flip a coin. If you lose the flip, sacrifice this enchantment.
//! Oracle: Whenever a player casts a spell, that player flips a coin. If they lose the flip, counter that spell.
//! Set: JUD #97 — Judgment | Scryfall ID: 5dae5e16-d2fc-488c-9c53-d35c377d6a00 | Oracle ID: f0864dbb-4d91-4282-a774-7b225e649ca8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PLANAR_CHAOS,
    oracle_id = "f0864dbb-4d91-4282-a774-7b225e649ca8",
    scryfall_id = "5dae5e16-d2fc-488c-9c53-d35c377d6a00",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Planar Chaos",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
