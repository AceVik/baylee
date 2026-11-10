//! Aven Warcraft — {2}{W} — Instant
//! Oracle: Creatures you control get +0/+2 until end of turn.
//! Oracle: Threshold — If there are seven or more cards in your graveyard, choose a color. Creatures you control also gain protection from the chosen color until end of turn.
//! Set: JUD #2 — Judgment | Scryfall ID: 1fd5d21a-b151-4fea-ac0d-6659af131bf9 | Oracle ID: c152f829-0452-407b-8020-023bf3d75c74
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AVEN_WARCRAFT,
    oracle_id = "c152f829-0452-407b-8020-023bf3d75c74",
    scryfall_id = "1fd5d21a-b151-4fea-ac0d-6659af131bf9",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Aven Warcraft",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
