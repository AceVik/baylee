//! Memorial to Unity — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {2}{G}, {T}, Sacrifice this land: Look at the top five cards of your library. You may reveal a creature card from among them and put it into your hand. Then put the rest on the bottom of your library in a random order.
//! Set: DOM #245 — Dominaria | Scryfall ID: 994d903b-0d54-4af4-898f-213e716e9ed4 | Oracle ID: a74494ef-aa35-4830-9b4c-47bff5270efc

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MEMORIAL_TO_UNITY,
    oracle_id = "a74494ef-aa35-4830-9b4c-47bff5270efc",
    scryfall_id = "994d903b-0d54-4af4-898f-213e716e9ed4",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Memorial to Unity",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        activated!(
            cost!("{2}{G}", TapSelf, SacrificeSelf),
            &[Effect::LookAtTopFind {
                count: 5,
                filter: &Filter::CREATURE,
                random: true
            }]
        )
    ],
);
