//! Mind Swords — {1}{B} — Sorcery
//! Oracle: If you control a Swamp, you may sacrifice a creature rather than pay this spell's mana cost.
//! Oracle: Each player exiles two cards from their hand.
//! Set: NEM #60 — Nemesis | Scryfall ID: 3d6d91df-008b-48f2-a84f-550702fbcdb3 | Oracle ID: 4af20f8c-2ba0-49d5-a64e-d3668cec30cd
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIND_SWORDS,
    oracle_id = "4af20f8c-2ba0-49d5-a64e-d3668cec30cd",
    scryfall_id = "3d6d91df-008b-48f2-a84f-550702fbcdb3",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Mind Swords",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
