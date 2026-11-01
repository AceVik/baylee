//! Decree of Savagery — {7}{G}{G} — Instant
//! Oracle: Put four +1/+1 counters on each creature you control.
//! Oracle: Cycling {4}{G}{G} ({4}{G}{G}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, you may put four +1/+1 counters on target creature.
//! Set: AFC #156 — Forgotten Realms Commander | Scryfall ID: ec6d7db1-eeb2-4e9e-9a4f-488595bbcf1c | Oracle ID: 96a6a8d4-6077-470f-984e-b576a24557a2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DECREE_OF_SAVAGERY,
    oracle_id = "96a6a8d4-6077-470f-984e-b576a24557a2",
    scryfall_id = "ec6d7db1-eeb2-4e9e-9a4f-488595bbcf1c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Decree of Savagery",
        mana_cost = mana!("{7}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
