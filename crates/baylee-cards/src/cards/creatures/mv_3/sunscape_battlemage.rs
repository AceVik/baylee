//! Sunscape Battlemage — {2}{W} — Creature — Human Wizard
//! Oracle: Kicker {1}{G} and/or {2}{U} (You may pay an additional {1}{G} and/or {2}{U} as you cast this spell.)
//! Oracle: When this creature enters, if it was kicked with its {1}{G} kicker, destroy target creature with flying.
//! Oracle: When this creature enters, if it was kicked with its {2}{U} kicker, draw two cards.
//! Set: DDE #40 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: 679e3d63-8902-458e-a9a3-cefe14f6b415 | Oracle ID: c8fd2232-e499-4889-a2ca-e486ea87d02b
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SUNSCAPE_BATTLEMAGE,
    oracle_id = "c8fd2232-e499-4889-a2ca-e486ea87d02b",
    scryfall_id = "679e3d63-8902-458e-a9a3-cefe14f6b415",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    faces = &[face!(
        name = "Sunscape Battlemage",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
