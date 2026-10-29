//! Gempalm Sorcerer — {2}{U} — Creature — Human Wizard Sorcerer
//! Oracle: Cycling {2}{U} ({2}{U}, Discard this card: Draw a card.)
//! Oracle: When you cycle this card, Wizard creatures gain flying until end of turn.
//! Set: LGN #39 — Legions | Scryfall ID: 67bda65b-2e26-4531-9f6a-952df314c8f7 | Oracle ID: c3162232-b713-44be-b2af-29d801279ab2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GEMPALM_SORCERER,
    oracle_id = "c3162232-b713-44be-b2af-29d801279ab2",
    scryfall_id = "67bda65b-2e26-4531-9f6a-952df314c8f7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Gempalm Sorcerer",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[
            subtypes::creature::HUMAN,
            subtypes::creature::WIZARD,
            subtypes::creature::SORCERER
        ],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
