//! Stormscape Battlemage — {2}{U} — Creature — Metathran Wizard
//! Oracle: Kicker {W} and/or {2}{B} (You may pay an additional {W} and/or {2}{B} as you cast this spell.)
//! Oracle: When this creature enters, if it was kicked with its {W} kicker, you gain 3 life.
//! Oracle: When this creature enters, if it was kicked with its {2}{B} kicker, destroy target nonblack creature. That creature can't be regenerated.
//! Set: C13 #58 — Commander 2013 | Scryfall ID: e8bfb971-0ff4-4413-907f-047fb84dfba9 | Oracle ID: 38ee748d-adcd-41df-9b23-d2a34829784c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::STORMSCAPE_BATTLEMAGE,
    oracle_id = "38ee748d-adcd-41df-9b23-d2a34829784c",
    scryfall_id = "e8bfb971-0ff4-4413-907f-047fb84dfba9",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    faces = &[face!(
        name = "Stormscape Battlemage",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::METATHRAN, subtypes::creature::WIZARD],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
