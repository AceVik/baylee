//! Fight or Flight — {3}{W} — Enchantment
//! Oracle: At the beginning of combat on each opponent's turn, separate all creatures that player controls into two piles. Only creatures in the pile of their choice can attack this turn.
//! Set: INV #16 — Invasion | Scryfall ID: 46bde162-3737-4b93-a27a-63b909a4183d | Oracle ID: 2fc42640-983a-49ca-9020-7756135077b4
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FIGHT_OR_FLIGHT,
    oracle_id = "2fc42640-983a-49ca-9020-7756135077b4",
    scryfall_id = "46bde162-3737-4b93-a27a-63b909a4183d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Fight or Flight",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
