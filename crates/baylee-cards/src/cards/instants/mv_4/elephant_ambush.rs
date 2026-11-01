//! Elephant Ambush — {2}{G}{G} — Instant
//! Oracle: Create a 3/3 green Elephant creature token.
//! Oracle: Flashback {6}{G}{G} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #240 — Odyssey | Scryfall ID: b4abdf1c-a0a9-4e1d-b448-58742830f767 | Oracle ID: 09386931-59bd-481f-bccc-32e8a8abc0f9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ELEPHANT_AMBUSH,
    oracle_id = "09386931-59bd-481f-bccc-32e8a8abc0f9",
    scryfall_id = "b4abdf1c-a0a9-4e1d-b448-58742830f767",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Elephant Ambush",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
