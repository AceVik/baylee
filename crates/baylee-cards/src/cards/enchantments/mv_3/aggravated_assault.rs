//! Aggravated Assault — {2}{R} — Enchantment
//! Oracle: {3}{R}{R}: Untap all creatures you control. After this main phase, there is an additional combat phase followed by an additional main phase. Activate only as a sorcery.
//! Set: ONS #185 — Onslaught | Scryfall ID: c99c5707-d5f2-4675-bfca-e801e6b0f627 | Oracle ID: 20129459-a386-41eb-899d-1aede3427300
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AGGRAVATED_ASSAULT,
    oracle_id = "20129459-a386-41eb-899d-1aede3427300",
    scryfall_id = "c99c5707-d5f2-4675-bfca-e801e6b0f627",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Aggravated Assault",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
