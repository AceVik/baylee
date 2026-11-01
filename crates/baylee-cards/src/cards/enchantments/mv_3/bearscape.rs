//! Bearscape — {1}{G}{G} — Enchantment
//! Oracle: {1}{G}, Exile two cards from your graveyard: Create a 2/2 green Bear creature token.
//! Set: ODY #229 — Odyssey | Scryfall ID: 3284b61a-bd95-4846-ad3f-903cd1158867 | Oracle ID: 7e481e22-4122-4259-8978-4727eb307195
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::BEARSCAPE,
    oracle_id = "7e481e22-4122-4259-8978-4727eb307195",
    scryfall_id = "3284b61a-bd95-4846-ad3f-903cd1158867",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Bearscape",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
