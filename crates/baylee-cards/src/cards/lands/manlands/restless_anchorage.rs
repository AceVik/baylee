//! Restless Anchorage — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W} or {U}.
//! Oracle: {1}{W}{U}: Until end of turn, this land becomes a 2/3 white and blue Bird creature with flying. It's still a land.
//! Oracle: Whenever this land attacks, create a Map token.
//! Set: FRC #81 — Reality Fracture Commander | Scryfall ID: 109900a9-6dcb-4f89-b84c-0ce0a0175e8f | Oracle ID: 91320daf-f69c-4350-b0fc-4bb37a6904b1
// IMPLEMENTED — tapped entry, white/blue mana, temporary Bird animation,
// and an attack trigger creating a Map with its explore activation.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::RESTLESS_ANCHORAGE,
    oracle_id = "91320daf-f69c-4350-b0fc-4bb37a6904b1",
    scryfall_id = "109900a9-6dcb-4f89-b84c-0ce0a0175e8f",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Restless Anchorage",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])]),
        activated!(
            cost!("{1}{W}{U}"),
            &[
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::CREATURE),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(creature::BIRD),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetColor(ColorSet::from_slice(&[Color::White, Color::Blue])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddKeyword(KeywordSet::FLYING),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(2, 3),
                    Duration::UntilEndOfTurn,
                ),
            ],
        ),
        triggered!(
            Trigger::Attacks(&Filter::This),
            &[Effect::CreateToken {
                token: &crate::tokens::MAP
            }],
        ),
    ],
);
