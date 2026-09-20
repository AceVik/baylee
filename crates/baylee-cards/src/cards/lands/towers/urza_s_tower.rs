//! Urza's Tower — (no cost) — Land — Urza's Tower
//! Oracle: {T}: Add {C}. If you control an Urza's Mine and an Urza's Power-Plant, add {C}{C}{C} instead.
//! Set: CMM #1053 — Commander Masters | Scryfall ID: 1e9f09b3-dd2d-4ba9-a57e-4f3c1793f752 | Oracle ID: 32fbb638-ab14-4e8b-a07a-d4c44e3496f2
// PARTIAL — {T}: Add {C} built; the "instead" clause is not expressible, see
// the NOT SUPPORTED line beside the ability.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URZA_S_TOWER,
    oracle_id = "32fbb638-ab14-4e8b-a07a-d4c44e3496f2",
    scryfall_id = "1e9f09b3-dd2d-4ba9-a57e-4f3c1793f752",
    faces = &[face!(
        name = "Urza's Tower",
        types = TypeSet::LAND,
        subtypes = &[subtypes::land::URZA_S, subtypes::land::TOWER],
    ),],
    coverage = Coverage::Partial(
        "the 'instead' clause: the printed condition is two counts joined by 'and', \
         and Condition offers only one ControlCount(filter, n) over a single filter",
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "If you control an Urza's Mine and an Urza's
        // Power-Plant, add {C}{C}{C} instead." — the clause is a conjunction
        // of two separate counts ("you control a Mine" AND "you control a
        // Power-Plant"), and the nearest variant,
        // `Condition::ControlCount(filter, n)`, counts one filter once. No
        // single filter answers it: a filter for "a Mine or a Power-Plant"
        // counted at two is also satisfied by two Mines, and a filter for a
        // permanent that is both matches neither card.
    ],
);
