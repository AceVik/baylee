//! Lure of Prey — {2}{G}{G} — Instant
//! Oracle: Cast this spell only if an opponent cast a creature spell this turn.
//! Oracle: You may put a green creature card from your hand onto the battlefield.
//! Set: MIR #227 — Mirage | Scryfall ID: 7165f1d8-7b31-4a81-aab4-c6cd4ff2e67d | Oracle ID: 3c83ea11-7141-41a2-ba77-1ed2722dfe74
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LURE_OF_PREY,
    oracle_id = "3c83ea11-7141-41a2-ba77-1ed2722dfe74",
    scryfall_id = "7165f1d8-7b31-4a81-aab4-c6cd4ff2e67d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Lure of Prey",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
