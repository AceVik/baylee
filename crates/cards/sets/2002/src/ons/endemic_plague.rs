//! Endemic Plague — {3}{B} — Sorcery
//! Oracle: As an additional cost to cast this spell, sacrifice a creature.
//! Oracle: Destroy all creatures that share a creature type with the sacrificed creature. They can't be regenerated.
//! Set: ONS #142 — Onslaught | Scryfall ID: 15326971-a53b-45f2-8f1d-1b82935286e1 | Oracle ID: db982577-1c75-4bc9-ab15-1888ea0be16d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENDEMIC_PLAGUE,
    oracle_id = "db982577-1c75-4bc9-ab15-1888ea0be16d",
    scryfall_id = "15326971-a53b-45f2-8f1d-1b82935286e1",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Endemic Plague",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
