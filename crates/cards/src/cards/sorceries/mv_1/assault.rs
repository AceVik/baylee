//! Assault // Battery — {R} — Sorcery // Sorcery
//! Oracle: Assault deals 2 damage to any target.
//! Oracle: Create a 3/3 green Elephant creature token.
//! Set: DMR #210 — Dominaria Remastered | Scryfall ID: 9b2ca0a2-0a18-4d9e-b953-52018af3c65b | Oracle ID: fcdeb83f-531c-4087-9254-7f19eb065f00
//! Face: Assault — {R} — Sorcery
//! Face: Battery — {3}{G} — Sorcery
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ASSAULT,
    oracle_id = "fcdeb83f-531c-4087-9254-7f19eb065f00",
    scryfall_id = "9b2ca0a2-0a18-4d9e-b953-52018af3c65b",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Red]),
    faces = &[
        face!(
            name = "Assault",
            mana_cost = mana!("{R}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Battery",
            mana_cost = mana!("{3}{G}"),
            types = TypeSet::SORCERY,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
