//! Seeds of Strength — {G}{W} — Instant
//! Oracle: Target creature gets +1/+1 until end of turn.
//! Oracle: Target creature gets +1/+1 until end of turn.
//! Oracle: Target creature gets +1/+1 until end of turn.
//! Set: RAV #227 — Ravnica: City of Guilds | Scryfall ID: bfa1ac13-adbd-439a-90b2-9c506aec0836 | Oracle ID: dd3d4d96-1e88-49e1-8276-7f62c22ad91a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEEDS_OF_STRENGTH,
    oracle_id = "dd3d4d96-1e88-49e1-8276-7f62c22ad91a",
    scryfall_id = "bfa1ac13-adbd-439a-90b2-9c506aec0836",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[face!(
        name = "Seeds of Strength",
        mana_cost = mana!("{G}{W}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
