//! Seize the Day — {3}{R} — Sorcery
//! Oracle: Untap target creature. After this main phase, there is an additional combat phase followed by an additional main phase.
//! Oracle: Flashback {2}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: MSC #166 — Marvel Super Heroes Commander | Scryfall ID: 9cb5d064-f474-4db9-b349-b9f71c587742 | Oracle ID: 2a6abd59-e448-46f1-9af8-bb9040645971
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEIZE_THE_DAY,
    oracle_id = "2a6abd59-e448-46f1-9af8-bb9040645971",
    scryfall_id = "9cb5d064-f474-4db9-b349-b9f71c587742",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Seize the Day",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
