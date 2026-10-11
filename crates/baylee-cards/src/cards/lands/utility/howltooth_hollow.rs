//! Howltooth Hollow — (no cost) — Land
//! Oracle: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Oracle: {B}, {T}: You may play the exiled card without paying its mana cost if each player has no cards in hand.
//! Set: LRW #269 — Lorwyn | Scryfall ID: 5c678643-6640-49e1-a5b0-2954123bcabc | Oracle ID: 463fc699-f4fc-4112-a6b3-6dcb642203e6
// IMPLEMENTED — hideaway, persistent inspection rights and conditional immediate free play.
// Behavior: baylee-engine/src/engine/card_tests/hidden_exile.rs.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::HOWLTOOTH_HOLLOW,
    oracle_id = "463fc699-f4fc-4112-a6b3-6dcb642203e6",
    scryfall_id = "5c678643-6640-49e1-a5b0-2954123bcabc",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Howltooth Hollow",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Black, 1)]),
        triggered!(Trigger::ETB, &[Effect::Hideaway(4)]),
        activated!(
            cost!("{B}", TapSelf),
            &[Effect::IfCondition {
                condition: Condition::EveryHandEmpty,
                then: &[Effect::MayPlayLinkedExile],
                otherwise: &[],
            }]
        ),
    ],
);
