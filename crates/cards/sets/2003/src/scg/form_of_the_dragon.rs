//! Form of the Dragon — {4}{R}{R}{R} — Enchantment
//! Oracle: At the beginning of your upkeep, this enchantment deals 5 damage to any target.
//! Oracle: At the beginning of each end step, your life total becomes 5.
//! Oracle: Creatures without flying can't attack you.
//! Set: 9ED #187 — Ninth Edition | Scryfall ID: 1cfc77f1-290a-4394-bc16-76f3659810f0 | Oracle ID: 7b389f2b-fddc-46fb-85d5-4470565323f3
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FORM_OF_THE_DRAGON,
    oracle_id = "7b389f2b-fddc-46fb-85d5-4470565323f3",
    scryfall_id = "1cfc77f1-290a-4394-bc16-76f3659810f0",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Form of the Dragon",
        mana_cost = mana!("{4}{R}{R}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
