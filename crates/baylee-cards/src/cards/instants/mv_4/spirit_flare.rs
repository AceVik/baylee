//! Spirit Flare — {3}{W} — Instant
//! Oracle: Tap target untapped creature you control. If you do, it deals damage equal to its power to target attacking or blocking creature an opponent controls.
//! Oracle: Flashback—{1}{W}, Pay 3 life. (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: TOR #15 — Torment | Scryfall ID: efdbe7c6-99ad-4c73-bffe-7bbdc6965854 | Oracle ID: d19a2c28-8d09-40bf-aeb1-e699672b009f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SPIRIT_FLARE,
    oracle_id = "d19a2c28-8d09-40bf-aeb1-e699672b009f",
    scryfall_id = "efdbe7c6-99ad-4c73-bffe-7bbdc6965854",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Spirit Flare",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
