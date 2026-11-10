//! Illuminate — {X}{R} — Sorcery
//! Oracle: Kicker {2}{R} and/or {3}{U} (You may pay an additional {2}{R} and/or {3}{U} as you cast this spell.)
//! Oracle: Illuminate deals X damage to target creature. If this spell was kicked with its {2}{R} kicker, it deals X damage to that creature's controller. If this spell was kicked with its {3}{U} kicker, you draw X cards.
//! Set: APC #63 — Apocalypse | Scryfall ID: ceef2761-7301-42de-8f54-49b8cd1e457b | Oracle ID: f944c245-08ff-41a8-bdfc-7d21a6a63eae
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ILLUMINATE,
    oracle_id = "f944c245-08ff-41a8-bdfc-7d21a6a63eae",
    scryfall_id = "ceef2761-7301-42de-8f54-49b8cd1e457b",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    faces = &[face!(
        name = "Illuminate",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
