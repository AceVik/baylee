//! Squee's Revenge — {1}{U}{R} — Sorcery
//! Oracle: Choose a number. Flip a coin that many times or until you lose a flip, whichever comes first. If you win all the flips, draw two cards for each flip.
//! Set: APC #123 — Apocalypse | Scryfall ID: 2b391ee3-c1cd-47bc-9540-977cbc32913e | Oracle ID: 3c9f3f26-339e-459e-9847-4e33aafb6f9b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SQUEE_S_REVENGE,
    oracle_id = "3c9f3f26-339e-459e-9847-4e33aafb6f9b",
    scryfall_id = "2b391ee3-c1cd-47bc-9540-977cbc32913e",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Squee's Revenge",
        mana_cost = mana!("{1}{U}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
