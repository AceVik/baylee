//! Agonizing Demise — {3}{B} — Instant
//! Oracle: Kicker {1}{R} (You may pay an additional {1}{R} as you cast this spell.)
//! Oracle: Destroy target nonblack creature. It can't be regenerated. If this spell was kicked, Agonizing Demise deals damage equal to that creature's power to the creature's controller.
//! Set: DDH #66 — Duel Decks: Ajani vs. Nicol Bolas | Scryfall ID: b7621aab-4839-45ef-bfa2-9aa2417bbec0 | Oracle ID: 41681a1f-0777-4b96-8c7e-d6a5d1b1c8fc
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AGONIZING_DEMISE,
    oracle_id = "41681a1f-0777-4b96-8c7e-d6a5d1b1c8fc",
    scryfall_id = "b7621aab-4839-45ef-bfa2-9aa2417bbec0",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Agonizing Demise",
        mana_cost = mana!("{3}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
