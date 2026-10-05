//! Avoid Fate — {G} — Instant
//! Oracle: Counter target instant or Aura spell that targets a permanent you control.
//! Set: TSB #73 — Time Spiral Timeshifted | Scryfall ID: 04c9f48d-2213-4f95-8eac-b23bd2d95e34 | Oracle ID: c04dd88f-fb7f-43be-b586-7fc5642073dc
// PARTIAL — nothing is built: "that targets a permanent you control" cannot
// be stated.
// NOT SUPPORTED: "Counter target instant or Aura spell that targets a
// permanent you control." — `TargetSpec::Spell(filter)` evaluates the filter
// against the spell's own characteristics, and no filter asks what a spell
// targets (`Filter::WithSingleTarget` only counts instances), so the
// restriction cannot be stated; an unrestricted instant-or-Aura counter would
// be strictly stronger, so the ability comes off the card.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::AVOID_FATE,
    oracle_id = "c04dd88f-fb7f-43be-b586-7fc5642073dc",
    scryfall_id = "04c9f48d-2213-4f95-8eac-b23bd2d95e34",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Avoid Fate",
        mana_cost = mana!("{G}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "no filter asks what a spell targets, so \"that targets a permanent \
         you control\" cannot be stated; an unrestricted counter would be \
         strictly stronger"
    ),
);
