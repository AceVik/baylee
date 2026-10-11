//! Shelldock Isle — (no cost) — Land
//! Oracle: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U}.
//! Oracle: {U}, {T}: You may play the exiled card without paying its mana cost if a library has twenty or fewer cards in it.
//! Set: LRW #272 — Lorwyn | Scryfall ID: 4216656e-90e8-45fc-a0f6-0d0d79d0a021 | Oracle ID: f748b2fb-6c2a-400a-8e96-fa4e4a1dfe80
// IMPLEMENTED — hideaway, persistent inspection rights and conditional immediate free play.
// Behavior: baylee-engine/src/engine/card_tests/hidden_exile.rs.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHELLDOCK_ISLE,
    oracle_id = "f748b2fb-6c2a-400a-8e96-fa4e4a1dfe80",
    scryfall_id = "4216656e-90e8-45fc-a0f6-0d0d79d0a021",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Shelldock Isle",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Blue, 1)]),
        triggered!(Trigger::ETB, &[Effect::Hideaway(4)]),
        activated!(
            cost!("{U}", TapSelf),
            &[Effect::IfCondition {
                condition: Condition::AnyLibraryAtMost(20),
                then: &[Effect::MayPlayLinkedExile],
                otherwise: &[],
            }]
        ),
    ],
);
