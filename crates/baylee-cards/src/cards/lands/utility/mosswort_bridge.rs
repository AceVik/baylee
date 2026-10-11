//! Mosswort Bridge — (no cost) — Land
//! Oracle: Hideaway 4 (When this land enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom in a random order.)
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G}.
//! Oracle: {G}, {T}: You may play the exiled card without paying its mana cost if creatures you control have total power 10 or greater.
//! Set: TDC #379 — Tarkir: Dragonstorm Commander | Scryfall ID: 1490c0dc-06d6-45ee-af3c-2935b0ab1233 | Oracle ID: 7cb9e29f-835f-4155-a2a5-4b778866c773
// IMPLEMENTED — hideaway, persistent inspection rights and conditional immediate free play.
// Behavior: baylee-engine/src/engine/card_tests/hidden_exile.rs.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MOSSWORT_BRIDGE,
    oracle_id = "7cb9e29f-835f-4155-a2a5-4b778866c773",
    scryfall_id = "1490c0dc-06d6-45ee-af3c-2935b0ab1233",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Mosswort Bridge",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Green, 1)]),
        triggered!(Trigger::ETB, &[Effect::Hideaway(4)]),
        activated!(
            cost!("{G}", TapSelf),
            &[Effect::IfCondition {
                condition: Condition::ControlledCreaturePowerAtLeast(10),
                then: &[Effect::MayPlayLinkedExile],
                otherwise: &[],
            }]
        ),
    ],
);
