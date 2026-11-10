//! Pain // Suffering — {B} — Sorcery // Sorcery
//! Oracle: Target player discards a card.
//! Oracle: Destroy target land.
//! Set: DMR #209 — Dominaria Remastered | Scryfall ID: 3d11164c-f27b-4cad-8620-d97ed384f0e6 | Oracle ID: 433fcba3-06b8-4f1a-a709-300838251439
//! Face: Pain — {B} — Sorcery
//! Face: Suffering — {3}{R} — Sorcery
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PAIN,
    oracle_id = "433fcba3-06b8-4f1a-a709-300838251439",
    scryfall_id = "3d11164c-f27b-4cad-8620-d97ed384f0e6",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[
        face!(
            name = "Pain",
            mana_cost = mana!("{B}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Suffering",
            mana_cost = mana!("{3}{R}"),
            types = TypeSet::SORCERY,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
