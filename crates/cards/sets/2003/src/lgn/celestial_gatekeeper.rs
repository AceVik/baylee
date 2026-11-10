//! Celestial Gatekeeper — {3}{W}{W} — Creature — Bird Cleric
//! Oracle: Flying
//! Oracle: When this creature dies, exile it, then return up to two target Bird and/or Cleric permanent cards from your graveyard to the battlefield.
//! Set: LGN #6 — Legions | Scryfall ID: 0b4dc1d3-53a1-411b-abf9-f5e4e80edc63 | Oracle ID: a7726d8c-7539-43bc-9614-418fa12c0912
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CELESTIAL_GATEKEEPER,
    oracle_id = "a7726d8c-7539-43bc-9614-418fa12c0912",
    scryfall_id = "0b4dc1d3-53a1-411b-abf9-f5e4e80edc63",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Celestial Gatekeeper",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::BIRD, subtypes::creature::CLERIC],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
