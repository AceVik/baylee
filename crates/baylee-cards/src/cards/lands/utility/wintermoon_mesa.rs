//! Wintermoon Mesa — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Tap two target lands.
//! Set: PCY #143 — Prophecy | Scryfall ID: f07144a6-6e47-4315-8353-f8958f014f41 | Oracle ID: a4a6f95e-856c-4eb5-82ba-b2406be22b23
// IMPLEMENTED — enters tapped, and {T}: Add {C}. The third ability is not
// expressible and is dropped; see NOT SUPPORTED below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINTERMOON_MESA,
    oracle_id = "a4a6f95e-856c-4eb5-82ba-b2406be22b23",
    scryfall_id = "f07144a6-6e47-4315-8353-f8958f014f41",
    faces = &[face!(
        name = "Wintermoon Mesa",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Partial(
        "the third ability taps two target lands, and an activated ability \
         states a single TargetSpec with no target count",
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "{2}, {T}, Sacrifice this land: Tap two target lands" — a
// count on a target sentence lives in `TargetReq`, which is reachable from
// `Spell`, `Triggered`, `Loyalty`, `SagaChapter` and a modal mode and never
// from `Activated`, whose `target` is a bare `TargetSpec`. So the ability
// would offer one land and tap one, and it comes off the card instead.
