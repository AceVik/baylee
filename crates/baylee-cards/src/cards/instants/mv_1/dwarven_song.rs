//! Dwarven Song — {R} — Instant
//! Oracle: One or more target creatures become red until end of turn.
//! Set: LEG #143 — Legends | Scryfall ID: 29a50f72-9524-4440-9380-9d3e0b693351 | Oracle ID: 29b7a220-cfcf-4b44-a5fb-737fd47d46cb
// PARTIAL — the colour change is dropped: only the first of one or more
// targets could be made red, because the effect that would do it binds
// `Filter::This` to the first target, and no effect sets the colour of every
// target.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DWARVEN_SONG,
    oracle_id = "29b7a220-cfcf-4b44-a5fb-737fd47d46cb",
    scryfall_id = "29a50f72-9524-4440-9380-9d3e0b693351",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "only the first of one or more targets can become red: Effect::continuous binds \
         Filter::This to the first target and no effect sets the colour of every target"
    ),
    faces = &[face!(
        name = "Dwarven Song",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "One or more target creatures become red until end of
    // turn." — no effect sets the colour of every target (CreateContinuousEffect's
    // Filter::This binds only the first target).
);
