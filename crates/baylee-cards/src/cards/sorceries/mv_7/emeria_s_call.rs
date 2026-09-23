//! Emeria's Call // Emeria, Shattered Skyclave — {4}{W}{W}{W} — Sorcery // Land
//! Oracle: Create two 4/4 white Angel Warrior creature tokens with flying. Non-Angel creatures you control gain indestructible until your next turn.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {W}.
//! Set: ZNR #12 — Zendikar Rising | Scryfall ID: c470539a-9cc7-4175-8f7c-c982b6072b6d | Oracle ID: 6ec2a242-9068-4ee2-8ac8-8341cc570f56
//! Face: Emeria's Call — {4}{W}{W}{W} — Sorcery
//! Face: Emeria, Shattered Skyclave —  — Land
// IMPLEMENTED — two 4/4 white flying Angel Warriors, indestructible for your
// non-Angels until your next turn, and the land's pay-3-or-enter-tapped plus
// its own mana ability.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::EMERIA_S_CALL,
    oracle_id = "6ec2a242-9068-4ee2-8ac8-8341cc570f56",
    scryfall_id = "c470539a-9cc7-4175-8f7c-c982b6072b6d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Emeria's Call",
            mana_cost = mana!("{4}{W}{W}{W}"),
            types = TypeSet::SORCERY,
        ),
        face!(
            name = "Emeria, Shattered Skyclave",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
            abilities = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(&[
        // The Warrior half of the type line is load-bearing on this very
        // card: the indestructibility below spares "non-Angel creatures",
        // and the pool's plain `ANGEL_4_4_WHITE_FLYING` would be the same
        // two bodies wearing the wrong type line.
        Effect::CreateTokenN {
            token: &generated_tokens::ANGEL_WARRIOR_4_4_WHITE_FLYING,
            amount: Amount::Fixed(2),
        },
        Effect::continuous(
            &Filter::And(&[
                Filter::CREATURE,
                Filter::ControlledByYou,
                Filter::Not(&Filter::HasSubtype(creature::ANGEL)),
            ]),
            Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE),
            Duration::UntilYourNextTurn,
        ),
    ])],
);
