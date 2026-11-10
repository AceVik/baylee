//! Flaring Pain — {1}{R} — Instant
//! Oracle: Damage can't be prevented this turn.
//! Oracle: Flashback {R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: JUD #89 — Judgment | Scryfall ID: eeb5c96a-1d16-459d-9968-ced9a8f1c520 | Oracle ID: cad19ad2-3ea9-43f5-bb5a-b0793402fab0
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLARING_PAIN,
    oracle_id = "cad19ad2-3ea9-43f5-bb5a-b0793402fab0",
    scryfall_id = "eeb5c96a-1d16-459d-9968-ced9a8f1c520",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Flaring Pain",
        mana_cost = mana!("{1}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
