//! Nature's Resurgence — {2}{G}{G} — Sorcery
//! Oracle: Each player draws a card for each creature card in their graveyard.
//! Set: 7ED #259 — Seventh Edition | Scryfall ID: 287f9f55-829d-4b29-b1d2-34d20d23b3d5 | Oracle ID: 276f49ee-7cf3-4a4d-9b14-74aedcbef69f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NATURE_S_RESURGENCE,
    oracle_id = "276f49ee-7cf3-4a4d-9b14-74aedcbef69f",
    scryfall_id = "287f9f55-829d-4b29-b1d2-34d20d23b3d5",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Nature's Resurgence",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
