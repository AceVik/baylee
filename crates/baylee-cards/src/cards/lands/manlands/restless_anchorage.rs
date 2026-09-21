//! Restless Anchorage — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W} or {U}.
//! Oracle: {1}{W}{U}: Until end of turn, this land becomes a 2/3 white and blue Bird creature with flying. It's still a land.
//! Oracle: Whenever this land attacks, create a Map token.
//! Set: FRC #81 — Reality Fracture Commander | Scryfall ID: 109900a9-6dcb-4f89-b84c-0ce0a0175e8f | Oracle ID: 91320daf-f69c-4350-b0fc-4bb37a6904b1
// PARTIAL — the land enters tapped, taps for {W} or {U}, and animates
// itself until end of turn; the attack trigger has no Map token to create.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::RESTLESS_ANCHORAGE,
    oracle_id = "91320daf-f69c-4350-b0fc-4bb37a6904b1",
    scryfall_id = "109900a9-6dcb-4f89-b84c-0ce0a0175e8f",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage =
        Coverage::Partial("the attack trigger's Map token does not exist in `crate::tokens`",),
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
                    Modifier::AddColor(ColorSet::from_slice(&[Color::White, Color::Blue])),
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
        // NOT SUPPORTED: "Whenever this land attacks, create a Map token." —
        // the pool has no Map token for `crate::tokens` to hand out, and a
        // card file may not define one (`no_card_file_defines_its_own_token`);
        // the token's own explore ability has no variant in this DSL either.
    ],
);
