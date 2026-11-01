//! Oath of Scholars — {3}{U} — Enchantment
//! Oracle: At the beginning of each player's upkeep, that player chooses target player who has more cards in hand than they do and is their opponent. The first player may discard their hand and draw three cards.
//! Set: EXO #42 — Exodus | Scryfall ID: d61376ad-21c8-4d34-b37d-ed60877f5d4a | Oracle ID: da4a5129-b005-4fc6-b175-0416627881db
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::OATH_OF_SCHOLARS,
    oracle_id = "da4a5129-b005-4fc6-b175-0416627881db",
    scryfall_id = "d61376ad-21c8-4d34-b37d-ed60877f5d4a",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Oath of Scholars",
        mana_cost = mana!("{3}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
