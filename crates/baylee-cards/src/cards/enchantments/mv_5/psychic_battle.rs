//! Psychic Battle — {3}{U}{U} — Enchantment
//! Oracle: Whenever a player chooses one or more targets, each player reveals the top card of their library. The player who reveals the card with the greatest mana value may change the target or targets. If two or more cards are tied for greatest, the target or targets remain unchanged. Changing targets this way doesn't trigger abilities of permanents named Psychic Battle.
//! Set: INV #68 — Invasion | Scryfall ID: 8758ca24-e613-43bf-be58-4cf557f82d0c | Oracle ID: 79f0d763-5a71-470a-867f-86fc72e9db6d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PSYCHIC_BATTLE,
    oracle_id = "79f0d763-5a71-470a-867f-86fc72e9db6d",
    scryfall_id = "8758ca24-e613-43bf-be58-4cf557f82d0c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Psychic Battle",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
