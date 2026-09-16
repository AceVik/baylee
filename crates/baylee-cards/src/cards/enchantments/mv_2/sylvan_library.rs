//! Sylvan Library — {1}{G} — Enchantment
//! Oracle: At the beginning of your draw step, you may draw two additional cards. If you do, choose two cards in your hand drawn this turn. For each of those cards, pay 4 life or put the card on top of your library.
//! Set: DMR #179 — Dominaria Remastered | Scryfall ID: 6ada256f-2e55-4c1f-b4d3-d7b10b498956 | Oracle ID: 92eed395-62ca-4293-882b-8565c40daab5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SYLVAN_LIBRARY,
    oracle_id = "92eed395-62ca-4293-882b-8565c40daab5",
    scryfall_id = "6ada256f-2e55-4c1f-b4d3-d7b10b498956",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Sylvan Library",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::ENCHANTMENT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
