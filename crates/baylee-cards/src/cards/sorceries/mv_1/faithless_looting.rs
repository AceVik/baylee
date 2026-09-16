//! Faithless Looting — {R} — Sorcery
//! Oracle: Draw two cards, then discard two cards.
//! Oracle: Flashback {2}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: SOC #244 — Secrets of Strixhaven Commander | Scryfall ID: fc019ffa-4461-4f3d-ab8d-e4d20e77ca0c | Oracle ID: 3d6fa57a-aa53-4b5c-b8af-a7612c823117
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FAITHLESS_LOOTING,
    oracle_id = "3d6fa57a-aa53-4b5c-b8af-a7612c823117",
    scryfall_id = "fc019ffa-4461-4f3d-ab8d-e4d20e77ca0c",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Faithless Looting",
        mana_cost = mana!("{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
