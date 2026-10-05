//! Sylvan Paradise — {G} — Instant
//! Oracle: One or more target creatures become green until end of turn.
//! Set: LEG #208 — Legends | Scryfall ID: f323c3bb-cece-4035-b1a7-c4817cf7a08c | Oracle ID: 91693cad-3233-440a-b2f0-fe4d8b17ed42
// PARTIAL — the colour change is dropped: only the first of one or more
// targets could be made green, because the effect that would do it binds
// `Filter::This` to the first target, and no effect sets the colour of every
// target.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SYLVAN_PARADISE,
    oracle_id = "91693cad-3233-440a-b2f0-fe4d8b17ed42",
    scryfall_id = "f323c3bb-cece-4035-b1a7-c4817cf7a08c",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "only the first of one or more targets can become green: Effect::continuous binds \
         Filter::This to the first target and no effect sets the colour of every target"
    ),
    faces = &[face!(
        name = "Sylvan Paradise",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "One or more target creatures become green until end of
    // turn." — no effect sets the colour of every target (CreateContinuousEffect's
    // Filter::This binds only the first target).
);
