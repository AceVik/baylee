//! Honorable Passage — {1}{W} — Instant
//! Oracle: The next time a source of your choice would deal damage to any target this turn, prevent that damage. If damage from a red source is prevented this way, Honorable Passage deals that much damage to the source's controller.
//! Set: TSB #9 — Time Spiral Timeshifted | Scryfall ID: b07d20d7-4fdc-4104-9a9f-5fd2d3aa7b66 | Oracle ID: 66805334-1015-4b41-ba9b-116de94b0744
// PARTIAL — Effect::PreventNextFromChosenSource prevents the next damage a chosen source would deal to you rather than any target, and does not check whether the source is red to deal that much damage to the source's controller; in play, Honorable Passage prevents the next damage a chosen source would deal to you this turn.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HONORABLE_PASSAGE,
    oracle_id = "66805334-1015-4b41-ba9b-116de94b0744",
    scryfall_id = "b07d20d7-4fdc-4104-9a9f-5fd2d3aa7b66",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "Effect::PreventNextFromChosenSource prevents the next damage a chosen source would deal to you rather than any target, and does not check whether the source is red to deal that much damage to the source's controller; in play, Honorable Passage prevents the next damage a chosen source would deal to you this turn.",
    ),
    faces = &[face!(
        name = "Honorable Passage",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(&[Effect::PreventNextFromChosenSource {
        sources: &Filter::Any,
        combat_only: false,
        all_but: 0,
        gain_life: false,
    }])],
);
