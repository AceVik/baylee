//! Telekinetic Bonds — {2}{U}{U}{U} — Enchantment
//! Oracle: Whenever a player discards a card, you may pay {1}{U}. If you do, you may tap or untap target permanent.
//! Set: JUD #52 — Judgment | Scryfall ID: d68fad1d-a517-433a-b939-d0635e8f5535 | Oracle ID: 708a76cc-d4ab-4dfd-98dd-815f5bc99364
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TELEKINETIC_BONDS,
    oracle_id = "708a76cc-d4ab-4dfd-98dd-815f5bc99364",
    scryfall_id = "d68fad1d-a517-433a-b939-d0635e8f5535",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Telekinetic Bonds",
        mana_cost = mana!("{2}{U}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
