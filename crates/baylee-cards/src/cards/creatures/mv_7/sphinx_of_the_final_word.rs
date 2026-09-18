//! Sphinx of the Final Word — {5}{U}{U} — Creature — Sphinx
//! Oracle: This spell can't be countered.
//! Oracle: Flying
//! Oracle: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
//! Oracle: Instant and sorcery spells you control can't be countered.
//! Set: FDN #747 — Foundations | Scryfall ID: 6071239e-0b85-4c54-bef8-d9456eb8d8fc | Oracle ID: d4246e4d-390d-4925-a5a8-89cd096a237c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPHINX_OF_THE_FINAL_WORD,
    oracle_id = "d4246e4d-390d-4925-a5a8-89cd096a237c",
    scryfall_id = "6071239e-0b85-4c54-bef8-d9456eb8d8fc",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Sphinx of the Final Word",
        mana_cost = mana!("{5}{U}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPHINX],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
