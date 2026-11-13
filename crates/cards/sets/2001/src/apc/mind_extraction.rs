//! Mind Extraction — {2}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Target player reveals their hand and discards all cards of each of the sacrificed creature's colors.
//! Set: APC #42 — Apocalypse | Scryfall ID: 7d77ddcc-e66b-4036-8a55-ec42953918d1 | Oracle ID: 0077740a-528b-4ee3-b331-fac321b95302
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIND_EXTRACTION,
    oracle_id = "0077740a-528b-4ee3-b331-fac321b95302",
    scryfall_id = "7d77ddcc-e66b-4036-8a55-ec42953918d1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Mind Extraction",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
