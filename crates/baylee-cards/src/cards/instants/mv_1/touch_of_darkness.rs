//! Touch of Darkness — {B} — Instant
//! Oracle: One or more target creatures become black until end of turn.
//! Set: LEG #122 — Legends | Scryfall ID: eda7177f-1354-4008-aaaa-2c8b823ed5e9 | Oracle ID: 73648693-c67f-454b-83e2-1d1787437a2a
// PARTIAL — the colour change is dropped: only the first of one or more
// targets could be made black, because the effect that would do it binds
// `Filter::This` to the first target, and no effect sets the colour of every
// target.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOUCH_OF_DARKNESS,
    oracle_id = "73648693-c67f-454b-83e2-1d1787437a2a",
    scryfall_id = "eda7177f-1354-4008-aaaa-2c8b823ed5e9",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "only the first of one or more targets can become black: Effect::continuous binds \
         Filter::This to the first target and no effect sets the colour of every target"
    ),
    faces = &[face!(
        name = "Touch of Darkness",
        mana_cost = mana!("{B}"),
        types = TypeSet::INSTANT,
    ),],
    // NOT SUPPORTED: "One or more target creatures become black until end of
    // turn." — no effect sets the colour of every target (CreateContinuousEffect's
    // Filter::This binds only the first target).
);
