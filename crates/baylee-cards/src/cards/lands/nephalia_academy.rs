//! Nephalia Academy — (no cost) — Land
//! Oracle: If a spell or ability an opponent controls causes you to discard a card, you may reveal that card and put it on top of your library instead of putting it anywhere else.
//! Oracle: {T}: Add {C}.
//! Set: EMN #205 — Eldritch Moon | Scryfall ID: 9243eb5b-905e-446f-9ee0-7557e3b31db3 | Oracle ID: 3b7e7a11-bf59-413d-8796-640d17c2c1c6
// PARTIAL — {T}: Add {C} built; the discard replacement has no DSL shape.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::NEPHALIA_ACADEMY,
    oracle_id = "3b7e7a11-bf59-413d-8796-640d17c2c1c6",
    scryfall_id = "9243eb5b-905e-446f-9ee0-7557e3b31db3",
    faces = &[face!(name = "Nephalia Academy", types = TypeSet::LAND,),],
    // NOT SUPPORTED: "If a spell or ability an opponent controls causes you
    // to discard a card, you may reveal that card and put it on top of your
    // library instead of putting it anywhere else." — `ReplacementRule` has
    // no variant that replaces a discard, and `Trigger` has no event for a
    // card being discarded.
    coverage = Coverage::Partial("the discard-replacement clause has no DSL variant"),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);
