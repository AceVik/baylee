//! Thunderscape Battlemage — {2}{R} — Creature — Human Wizard
//! Oracle: Kicker {1}{B} and/or {G} (You may pay an additional {1}{B} and/or {G} as you cast this spell.)
//! Oracle: When this creature enters, if it was kicked with its {1}{B} kicker, target player discards two cards.
//! Oracle: When this creature enters, if it was kicked with its {G} kicker, destroy target enchantment.
//! Set: DDE #41 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: ddb1475d-abee-4eec-8615-bf5e286afb4a | Oracle ID: 9aa66e7d-d5e3-4c48-9c68-b61ff2af0b1a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::THUNDERSCAPE_BATTLEMAGE,
    oracle_id = "9aa66e7d-d5e3-4c48-9c68-b61ff2af0b1a",
    scryfall_id = "ddb1475d-abee-4eec-8615-bf5e286afb4a",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Red]),
    faces = &[face!(
        name = "Thunderscape Battlemage",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
