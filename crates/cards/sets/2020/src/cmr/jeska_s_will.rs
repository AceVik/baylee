//! Jeska's Will — {2}{R} — Sorcery
//! Oracle: Choose one. If you control a commander as you cast this spell, you may choose both instead.
//! Oracle: • Add {R} for each card in target opponent's hand.
//! Oracle: • Exile the top three cards of your library. You may play them this turn.
//! Set: MKC #156 — Murders at Karlov Manor Commander | Scryfall ID: 99e0c371-1024-4432-9fd9-3bc29c8d38e4 | Oracle ID: 0fd114c4-092b-4e28-b0dc-ef529f3bc73e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::JESKA_S_WILL,
    oracle_id = "0fd114c4-092b-4e28-b0dc-ef529f3bc73e",
    scryfall_id = "99e0c371-1024-4432-9fd9-3bc29c8d38e4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Jeska's Will",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
