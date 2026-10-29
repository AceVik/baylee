//! Verduran Emissary — {2}{G} — Creature — Human Wizard
//! Oracle: Kicker {1}{R} (You may pay an additional {1}{R} as you cast this spell.)
//! Oracle: When this creature enters, if it was kicked, destroy target artifact. It can't be regenerated.
//! Set: DDE #43 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: cf0a1951-4482-4ba0-b29e-2091adde0628 | Oracle ID: 915bd598-c3dd-4982-bdcc-5904a6f17436
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VERDURAN_EMISSARY,
    oracle_id = "915bd598-c3dd-4982-bdcc-5904a6f17436",
    scryfall_id = "cf0a1951-4482-4ba0-b29e-2091adde0628",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[face!(
        name = "Verduran Emissary",
        mana_cost = mana!("{2}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
