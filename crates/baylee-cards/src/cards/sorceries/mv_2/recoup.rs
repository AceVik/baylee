//! Recoup — {1}{R} — Sorcery
//! Oracle: Target sorcery card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost. (Mana cost includes color.)
//! Oracle: Flashback {3}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: DDK #63 — Duel Decks: Sorin vs. Tibalt | Scryfall ID: 77ac0e4e-71ae-4a4b-aa73-64079bd50308 | Oracle ID: f80d151c-7489-4189-8c5b-6d2a730d238e
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RECOUP,
    oracle_id = "f80d151c-7489-4189-8c5b-6d2a730d238e",
    scryfall_id = "77ac0e4e-71ae-4a4b-aa73-64079bd50308",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Recoup",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
