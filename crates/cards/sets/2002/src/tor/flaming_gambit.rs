//! Flaming Gambit — {X}{R} — Instant
//! Oracle: Flaming Gambit deals X damage to target player or planeswalker. That player or that planeswalker's controller may choose a creature they control and have Flaming Gambit deal that damage to it instead.
//! Oracle: Flashback {X}{R}{R} (You may cast this card from your graveyard for its flashback cost. Then exile it.)
//! Set: TOR #98 — Torment | Scryfall ID: fb7fd9b7-c394-4ab3-b945-b4aab694eb6a | Oracle ID: fab8c985-d7a9-41eb-9884-01128b97af5c
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FLAMING_GAMBIT,
    oracle_id = "fab8c985-d7a9-41eb-9884-01128b97af5c",
    scryfall_id = "fb7fd9b7-c394-4ab3-b945-b4aab694eb6a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[face!(
        name = "Flaming Gambit",
        mana_cost = mana!("{X}{R}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
