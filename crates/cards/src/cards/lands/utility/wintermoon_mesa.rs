//! Wintermoon Mesa — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}, Sacrifice this land: Tap two target lands.
//! Set: PCY #143 — Prophecy | Scryfall ID: f07144a6-6e47-4315-8353-f8958f014f41 | Oracle ID: a4a6f95e-856c-4eb5-82ba-b2406be22b23

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
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // "Two target lands" is exactly two, so the ability is not offered
        // while only one land is there to name.
        activated!(
            cost!("{2}", TapSelf, SacrificeSelf),
            &[Effect::TapTarget],
            targets = Some(TargetReq::exactly(TargetSpec::Object(&Filter::LAND), 2)),
        ),
    ],
);
