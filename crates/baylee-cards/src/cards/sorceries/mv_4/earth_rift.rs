//! Earth Rift — {3}{R} — Sorcery
//! Oracle: Destroy target land.
//! Oracle: Flashback {5}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: ODY #189 — Odyssey | Scryfall ID: e2e10742-77c9-4f91-81b2-37b2ac910f09 | Oracle ID: 888a5071-24b1-48ce-a97b-39f108976c6d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::EARTH_RIFT,
    oracle_id = "888a5071-24b1-48ce-a97b-39f108976c6d",
    scryfall_id = "e2e10742-77c9-4f91-81b2-37b2ac910f09",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Earth Rift",
        mana_cost = mana!("{3}{R}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
