//! Rust — {G} — Instant
//! Oracle: Counter target activated ability from an artifact source. (Mana abilities can't be targeted.)
//! Set: LEG #203 — Legends | Scryfall ID: ad4974c8-34c5-4290-b325-7586a67f6d56 | Oracle ID: 597ca66c-6bb1-4661-8ed9-8236f546e536
// PARTIAL — nothing is built: the artifact-source restriction cannot be stated.
// NOT SUPPORTED: "Counter target activated ability from an artifact source." —
// `TargetSpec::AbilityOnStack(filter)` matches the ability object itself,
// whose characteristics are the blank base, and no filter reads the ability's
// source; an unrestricted `Effect::CounterTargetAbility` would also counter
// triggered abilities and abilities from every other source, so the ability
// comes off the card rather than shipping stronger than it prints.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RUST,
    oracle_id = "597ca66c-6bb1-4661-8ed9-8236f546e536",
    scryfall_id = "ad4974c8-34c5-4290-b325-7586a67f6d56",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Rust",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "no filter reads an ability's source, so \"from an artifact source\" \
         cannot be stated; the target spec matches the ability object, whose \
         characteristics are blank"
    ),
);
