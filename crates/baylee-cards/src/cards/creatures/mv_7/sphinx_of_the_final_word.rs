//! Sphinx of the Final Word — {5}{U}{U} — Creature — Sphinx
//! Oracle: This spell can't be countered.
//! Oracle: Flying
//! Oracle: Hexproof (This creature can't be the target of spells or abilities your opponents control.)
//! Oracle: Instant and sorcery spells you control can't be countered.
//! Set: FDN #747 — Foundations | Scryfall ID: 6071239e-0b85-4c54-bef8-d9456eb8d8fc | Oracle ID: d4246e4d-390d-4925-a5a8-89cd096a237c
// PARTIAL — flying and hexproof sit on the face as the two keyword bits they are; the two "can't be countered" sentences are not expressible.

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
    keywords = KeywordSet::FLYING.union(KeywordSet::HEXPROOF),
    coverage = Coverage::Partial(
        "no Modifier makes a spell uncounterable, so both the card's own \"can't be countered\" and the one it grants to your instant and sorcery spells are missing"
    ),
);

// NOT SUPPORTED: "This spell can't be countered." — nothing in the DSL makes
// a spell on the stack uncounterable; `SpendRider::Uncounterable` is a rider
// on restricted mana (Cavern of Souls) and not a sentence a card can state
// about itself.
// NOT SUPPORTED: "Instant and sorcery spells you control can't be countered."
// — a static ability granting it would need a Modifier that grants
// uncounterability, and `Modifier` has no such variant (`AddKeyword` would
// double as it, but then the keyword bit would be a second name for a rule
// and no `//! Oracle:` line prints the word).
