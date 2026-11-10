//! Lightning Rift — {1}{R} — Enchantment
//! Oracle: Whenever a player cycles a card, you may pay {1}. If you do, this enchantment deals 2 damage to any target.
//! Set: DMR #129 — Dominaria Remastered | Scryfall ID: d1b731b2-70cb-4e94-8c9a-d694f617a521 | Oracle ID: ac3fb7ef-709c-40be-a75b-1b438708388d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LIGHTNING_RIFT,
    oracle_id = "ac3fb7ef-709c-40be-a75b-1b438708388d",
    scryfall_id = "d1b731b2-70cb-4e94-8c9a-d694f617a521",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Lightning Rift",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
