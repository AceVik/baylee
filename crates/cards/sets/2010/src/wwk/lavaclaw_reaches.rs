//! Lavaclaw Reaches — (no cost) — Land
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B} or {R}.
//! Oracle: {1}{B}{R}: Until end of turn, this land becomes a 2/2 black and red Elemental creature with "{X}: This creature gets +X/+0 until end of turn." It's still a land.
//! Set: UMA #245 — Ultimate Masters | Scryfall ID: 409fcd0c-8449-4623-8bf0-cd7ca0937a4a | Oracle ID: 340de307-982a-4b26-9dc0-99113dc766cd

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::LAVACLAW_REACHES,
    oracle_id = "340de307-982a-4b26-9dc0-99113dc766cd",
    scryfall_id = "409fcd0c-8449-4623-8bf0-cd7ca0937a4a",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Red]),
    faces = &[face!(
        name = "Lavaclaw Reaches",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::Tapped],
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana_choice(&[ManaColor::Black, ManaColor::Red])]),
        activated!(
            cost!("{1}{B}{R}"),
            &[
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
                    Modifier::SetColor(ColorSet::from_slice(&[Color::Black, Color::Red])),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::SetPT(2, 2),
                    Duration::UntilEndOfTurn,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::GrantActivated {
                        cost: cost!("{X}"),
                        mana_ability: false,
                        effects: &[Effect::PumpFilter {
                            filter: &Filter::This,
                            controlled_by: None,
                            power: Amount::X,
                            toughness: Amount::Fixed(0),
                            keywords: KeywordSet::EMPTY,
                            duration: Duration::UntilEndOfTurn,
                        }],
                    },
                    Duration::UntilEndOfTurn
                ),
            ],
        ),
    ],
);
