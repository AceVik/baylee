//! Strength in Numbers — {1}{G} — Instant
//! Oracle: Until end of turn, target creature gains trample and gets +X/+X, where X is the number of attacking creatures.
//! Set: TSR #233 — Time Spiral Remastered | Scryfall ID: 51d900d0-3f3a-4716-bcc7-b27a0cbbc339 | Oracle ID: 23195903-04e1-4461-a4ac-f0ce39f21c20
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STRENGTH_IN_NUMBERS,
    oracle_id = "23195903-04e1-4461-a4ac-f0ce39f21c20",
    scryfall_id = "51d900d0-3f3a-4716-bcc7-b27a0cbbc339",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Strength in Numbers",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
