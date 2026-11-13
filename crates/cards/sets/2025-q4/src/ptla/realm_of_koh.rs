//! Realm of Koh — (no cost) — Land
//! Oracle: This land enters tapped unless you control a basic land.
//! Oracle: {T}: Add {B}.
//! Oracle: {3}{B}, {T}: Create a 1/1 colorless Spirit creature token with "This token can't block or be blocked by non-Spirit creatures."
//! Set: TLA #276 — Avatar: The Last Airbender | Scryfall ID: 061ac694-610c-479f-b038-a4ef5270d5d7 | Oracle ID: bb9ce416-eef1-49e8-89a0-2b6837505070

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::REALM_OF_KOH,
    oracle_id = "bb9ce416-eef1-49e8-89a0-2b6837505070",
    scryfall_id = "061ac694-610c-479f-b038-a4ef5270d5d7",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Realm of Koh",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&Filter::YOUR_BASIC_LAND)],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        activated!(
            cost!("{3}{B}", TapSelf),
            &[Effect::CreateToken {
                token: &crate::tokens::SPIRIT_1_1_COLORLESS_SPIRIT_COMBAT
            }]
        ),
    ],
);
