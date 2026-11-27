//! Failure // Comply — {1}{U} — Instant // Sorcery
//! Oracle: Return target spell to its owner's hand.
//! Oracle: Aftermath (Cast this spell only from your graveyard. Then exile it.)
//! Oracle: Choose a card name. Until your next turn, your opponents can't cast spells with the chosen name.
//! Set: AKH #221 — Amonkhet | Scryfall ID: b9623c8c-01b4-4e8f-a5b9-eeea408ec027 | Oracle ID: ed8184f8-f6a7-4f97-9f4e-bf72077af754
//! Face: Failure — {1}{U} — Instant
//! Face: Comply — {W} — Sorcery
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::FAILURE,
    oracle_id = "ed8184f8-f6a7-4f97-9f4e-bf72077af754",
    scryfall_id = "b9623c8c-01b4-4e8f-a5b9-eeea408ec027",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    faces = &[
        face!(
            name = "Failure",
            mana_cost = mana!("{1}{U}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Comply",
            mana_cost = mana!("{W}"),
            types = TypeSet::SORCERY,
        ),
    ],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
