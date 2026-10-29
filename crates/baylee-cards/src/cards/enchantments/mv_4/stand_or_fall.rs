//! Stand or Fall — {3}{R} — Enchantment
//! Oracle: At the beginning of combat on your turn, for each defending player, separate all creatures that player controls into two piles and that player chooses one. Only creatures in the chosen piles can block this turn.
//! Set: INV #171 — Invasion | Scryfall ID: 60c34970-a106-490c-ac37-6156eb7f34ce | Oracle ID: 927d0085-9bd9-4454-8ac5-0e374237ed35
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STAND_OR_FALL,
    oracle_id = "927d0085-9bd9-4454-8ac5-0e374237ed35",
    scryfall_id = "60c34970-a106-490c-ac37-6156eb7f34ce",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Stand or Fall",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
