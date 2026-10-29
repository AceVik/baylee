//! Skeletal Scrying — {X}{B} — Instant
//! Oracle: As an additional cost to cast this spell, exile X cards from your graveyard.
//! Oracle: You draw X cards and you lose X life.
//! Set: C17 #125 — Commander 2017 | Scryfall ID: 6653ed3b-2a82-4e22-a9ca-18da5da94990 | Oracle ID: 561c68d8-b727-48d8-a832-2e42d1345fac
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SKELETAL_SCRYING,
    oracle_id = "561c68d8-b727-48d8-a832-2e42d1345fac",
    scryfall_id = "6653ed3b-2a82-4e22-a9ca-18da5da94990",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Skeletal Scrying",
        mana_cost = mana!("{X}{B}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
