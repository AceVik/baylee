//! Decree of Pain — {6}{B}{B} — Sorcery
//! Oracle: Destroy all creatures. They can't be regenerated. Draw a card for each creature destroyed this way.
//! Oracle: Cycling {3}{B}{B} ({3}{B}{B}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, all creatures get -2/-2 until end of turn.
//! Set: DSC #136 — Duskmourn: House of Horror Commander | Scryfall ID: bd594aeb-f5cc-4435-8ea8-c35c5824b58c | Oracle ID: 932668fa-d6e3-41c0-ad0c-8e0a00e68d11
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DECREE_OF_PAIN,
    oracle_id = "932668fa-d6e3-41c0-ad0c-8e0a00e68d11",
    scryfall_id = "bd594aeb-f5cc-4435-8ea8-c35c5824b58c",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Decree of Pain",
        mana_cost = mana!("{6}{B}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
