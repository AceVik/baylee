//! Aether Rift — {1}{R}{G} — Enchantment
//! Oracle: At the beginning of your upkeep, discard a card at random. If you discard a creature card this way, return it from your graveyard to the battlefield unless any player pays 5 life.
//! Set: INV #227 — Invasion | Scryfall ID: 692c186a-997c-4f7e-a339-bf84884e1019 | Oracle ID: 2577ef8e-d85e-47af-ac34-629f31992751
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AETHER_RIFT,
    oracle_id = "2577ef8e-d85e-47af-ac34-629f31992751",
    scryfall_id = "692c186a-997c-4f7e-a339-bf84884e1019",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(
        name = "Aether Rift",
        mana_cost = mana!("{1}{R}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
