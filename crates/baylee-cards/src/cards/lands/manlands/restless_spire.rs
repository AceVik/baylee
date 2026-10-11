//! Restless Spire — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {U} or {R}.
//! Oracle: {U}{R}: Until end of turn, this land becomes a 2/1 blue and red Elemental creature with "During your turn, this creature has first strike." It's still a land.
//! Oracle: Whenever this land attacks, scry 1.
//! Set: FRC #82 — Reality Fracture Commander | Scryfall ID: 30ecfd44-8dd4-4ae3-9076-14799f8a9a14 | Oracle ID: 0ca4e80e-c19c-4b74-b531-c5a4dc5a8ba9

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::RESTLESS_SPIRE,
    oracle_id = "0ca4e80e-c19c-4b74-b531-c5a4dc5a8ba9",
    scryfall_id = "30ecfd44-8dd4-4ae3-9076-14799f8a9a14",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Restless Spire",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Blue, ManaColor::Red])]),
        activated!(
            cost!("{U}{R}"),
            &[
                Effect::ModifySource {
                    modifier: Modifier::GrantStatic {
                        filter: &Filter::And(&[Filter::This, Filter::ControlledByActivePlayer]),
                        modifier: &Modifier::AddKeyword(KeywordSet::FIRST_STRIKE),
                    },
                    duration: Duration::UntilEndOfTurn
                },
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::ELEMENTAL),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetColor(ColorSet::from_slice(&[Color::Blue, Color::Red])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(2, 1),
                    Duration::UntilEndOfTurn,
                ),
            ]
        ),
        triggered!(Trigger::Attacks(&Filter::This), &[Effect::scry(1)]),
    ],
);
