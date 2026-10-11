//! Windbrisk Heights — (no cost) — Land
//! Oracle: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W}.
//! Oracle: {W}, {T}: You may play the exiled card without paying its mana cost if you attacked with three or more creatures this turn.
//! Set: TDC #411 — Tarkir: Dragonstorm Commander | Scryfall ID: df441bae-3b9c-46a9-8287-870c00d58a64 | Oracle ID: 3589bcfc-42b0-414a-adce-bc690dc631c8
// IMPLEMENTED — hideaway, persistent inspection rights and conditional immediate free play.
// Behavior: baylee-engine/src/engine/card_tests/hidden_exile.rs.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::WINDBRISK_HEIGHTS,
    oracle_id = "3589bcfc-42b0-414a-adce-bc690dc631c8",
    scryfall_id = "df441bae-3b9c-46a9-8287-870c00d58a64",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Windbrisk Heights",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        triggered!(Trigger::ETB, &[Effect::Hideaway(4)]),
        activated!(
            cost!("{W}", TapSelf),
            &[Effect::IfCondition {
                condition: Condition::AttackedWithAtLeast(3),
                then: &[Effect::MayPlayLinkedExile],
                otherwise: &[],
            }]
        ),
    ],
);
