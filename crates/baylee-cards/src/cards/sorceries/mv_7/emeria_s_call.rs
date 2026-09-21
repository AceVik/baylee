//! Emeria's Call // Emeria, Shattered Skyclave — {4}{W}{W}{W} — Sorcery // Land
//! Oracle: Create two 4/4 white Angel Warrior creature tokens with flying. Non-Angel creatures you control gain indestructible until your next turn.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {W}.
//! Set: ZNR #12 — Zendikar Rising | Scryfall ID: c470539a-9cc7-4175-8f7c-c982b6072b6d | Oracle ID: 6ec2a242-9068-4ee2-8ac8-8341cc570f56
//! Face: Emeria's Call — {4}{W}{W}{W} — Sorcery
//! Face: Emeria, Shattered Skyclave —  — Land
// PARTIAL — both halves play: two 4/4 white flying tokens, indestructible for
// your non-Angels until your next turn, and the land's pay-3-or-enter-tapped
// plus its own mana ability. What is short is the token's identity: the pool's
// 4/4 white flying Angel prints no Warrior subtype and has no Angel Warrior
// sibling in `crate::tokens`.

use crate::tokens;
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
    coverage = Coverage::Partial(
        "the two tokens are `tokens::ANGEL_4_4_WHITE_FLYING`, the pool's 4/4 \
         white flying Angel, which prints no Warrior subtype — no Angel \
         Warrior token has an id in `crate::tokens`",
    ),
    abilities = &[spell!(&[
        // NOT SUPPORTED: "4/4 white Angel Warrior … tokens with flying" —
        // the size, colour and flying are the printed ones, the creature
        // type is the Angel's alone.
        Effect::CreateTokenN {
            token: &tokens::ANGEL_4_4_WHITE_FLYING,
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
